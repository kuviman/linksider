use super::*;

#[derive(Deserialize)]
pub struct Controls {
    pub toggle: geng::Key,
    create: geng::MouseButton,
    delete: geng::MouseButton,
    choose: geng::Key,
    pick: geng::Key,
    grid: geng::Key,
    rotate: geng::Key,
}

#[derive(Deserialize)]
struct BrushWheelConfig {
    radius: f32,
    inner_radius: f32,
    color: Rgba<f32>,
}

#[derive(Deserialize)]
pub struct Config {
    default_fov: f32,
    index_size: f32,
    index_color: Rgba<f32>,
    grid_color: Rgba<f32>,
    brush_preview_opacity: f32,
    brush_wheel: BrushWheelConfig,
    autosave_timer: f64,
    pub controls: Controls,
}

enum BrushType {
    Entity(String),
    Tile(Tile),
    Powerup(Effect),
    Goal,
}

impl BrushType {
    fn tile_name(&self) -> String {
        match self {
            Self::Entity(name) => name.clone(),
            Self::Tile(tile) => format!("{tile:?}").to_lowercase(),
            Self::Powerup(effect) => format!("{effect:?}Power"),
            Self::Goal => "Goal".to_owned(),
        }
    }
}

struct Brush {
    angle: IntAngle,
    brush_type: BrushType,
}

impl Brush {
    fn rotation(&self) -> f32 {
        // TODO normalize angles in the codebase
        let angle = self.angle;
        match self.brush_type {
            BrushType::Entity(_) => angle,
            BrushType::Tile(_) => angle,
            BrushType::Powerup(_) => angle.rotate_counter_clockwise(),
            BrushType::Goal => angle,
        }
        .to_radians()
    }
    fn pick(state: &GameState, cell: vec2<i32>) -> Option<Self> {
        if let Some(tile) = state.tiles.get(&cell) {
            return Some(Self {
                angle: IntAngle::RIGHT,
                brush_type: BrushType::Tile(*tile),
            });
        }
        if let Some(entity) = state.entities.iter().find(|entity| entity.pos.cell == cell) {
            return Some(Self {
                angle: entity.pos.angle,
                brush_type: BrushType::Entity(entity.identifier.clone()),
            });
        }
        if let Some(powerup) = state
            .powerups
            .iter()
            .find(|powerup| powerup.pos.cell == cell)
        {
            return Some(Self {
                angle: powerup.pos.angle,
                brush_type: BrushType::Powerup(powerup.effect.clone()),
            });
        }
        if let Some(goal) = state.goals.iter().find(|goal| goal.pos.cell == cell) {
            return Some(Self {
                angle: goal.pos.angle,
                brush_type: BrushType::Goal,
            });
        }
        None
    }
}

struct BrushWheelItem {
    brush: Brush,
    pos: vec2<f32>,
    hovered: bool,
}

pub struct State<'a> {
    ctx: Context,
    title: String,
    framebuffer_size: vec2<f32>,
    config: Rc<Config>,
    game_state: &'a mut GameState,
    camera: Camera2d,
    level_mesh: renderer::LevelMesh,
    camera_controls: CameraControls,
    brush: Brush,
    brush_wheel_pos: Option<vec2<f32>>,
    path: std::path::PathBuf,
    history: Vec<GameState>,
    autosave_timer: Timer,
    show_grid: bool,
}

impl<'a> State<'a> {
    pub fn new(
        ctx: &Context,
        title: String,
        game_state: &'a mut GameState,
        path: impl AsRef<std::path::Path>,
    ) -> Self {
        let path = path.as_ref();
        let config = ctx.assets.config.editor.level.clone();
        Self {
            title,
            autosave_timer: Timer::new(),
            path: path.to_owned(),
            framebuffer_size: vec2::splat(1.0),
            camera: Camera2d {
                center: game_state.center(),
                rotation: 0.0,
                fov: config.default_fov,
            },
            config,
            level_mesh: ctx.renderer.level_mesh(&game_state),
            camera_controls: CameraControls::new(&ctx.geng, &ctx.assets.config.camera_controls),
            brush: Brush {
                angle: IntAngle::RIGHT,
                brush_type: BrushType::Entity("Player".to_owned()),
            },
            brush_wheel_pos: None,
            history: vec![game_state.clone()],
            game_state,
            show_grid: true,
            ctx: ctx.clone(),
        }
    }

    fn screen_to_tile(&self, screen_pos: vec2<f64>) -> vec2<i32> {
        let world_pos = self
            .camera
            .screen_to_world(self.framebuffer_size, screen_pos.map(|x| x as f32));
        world_pos.map(|x| x.floor() as i32)
    }

    fn create(&mut self, screen_pos: vec2<f64>) {
        self.delete(screen_pos);
        let cell = self.screen_to_tile(screen_pos);
        match &self.brush.brush_type {
            BrushType::Entity(name) => self.game_state.add_entity(
                name,
                &self.ctx.assets.logic_config.entities[name],
                Position {
                    cell,
                    angle: self.brush.angle,
                },
            ),
            BrushType::Tile(tile) => {
                self.game_state.tiles.insert(cell, *tile);
                self.level_mesh = self.ctx.renderer.level_mesh(&self.game_state);
            }
            BrushType::Powerup(effect) => {
                self.game_state.powerups.insert(Powerup {
                    id: self.game_state.id_gen.gen(),
                    pos: Position {
                        cell,
                        angle: self.brush.angle,
                    },
                    effect: effect.clone(),
                });
            }
            BrushType::Goal => self.game_state.goals.insert(Goal {
                id: self.game_state.id_gen.gen(),
                pos: Position {
                    cell,
                    angle: self.brush.angle,
                },
            }),
        }
    }

    fn delete(&mut self, screen_pos: vec2<f64>) {
        let tile = self.screen_to_tile(screen_pos);
        if self.game_state.tiles.remove(&tile).is_some() {
            self.level_mesh = self.ctx.renderer.level_mesh(&self.game_state);
        }
        self.game_state
            .entities
            .retain(|entity| entity.pos.cell != tile);
        self.game_state
            .powerups
            .retain(|entity| entity.pos.cell != tile);
        self.game_state
            .goals
            .retain(|entity| entity.pos.cell != tile);
    }

    fn brush_wheel(&self) -> Option<impl Iterator<Item = BrushWheelItem> + '_> {
        let center = self.brush_wheel_pos?;
        let entities = self
            .ctx
            .assets
            .logic_config
            .entities
            .keys()
            .map(|name| BrushType::Entity(name.clone()))
            .map(|brush_type| Brush {
                angle: IntAngle::RIGHT,
                brush_type,
            });
        let tiles = Tile::iter_variants()
            .map(BrushType::Tile)
            .map(|brush_type| Brush {
                angle: IntAngle::RIGHT,
                brush_type,
            });
        let powerups = Effect::iter_variants()
            .map(BrushType::Powerup)
            .map(|brush_type| Brush {
                angle: IntAngle::DOWN,
                brush_type,
            });
        let goal = Brush {
            angle: IntAngle::RIGHT,
            brush_type: BrushType::Goal,
        };

        let mut items: Vec<BrushWheelItem> = itertools::chain![entities, tiles, powerups, [goal]]
            .map(|brush| BrushWheelItem {
                brush,
                pos: vec2::ZERO,
                hovered: false,
            })
            .collect();
        let len = items.len();
        for (index, item) in items.iter_mut().enumerate() {
            item.pos = center
                + vec2(self.config.brush_wheel.radius, 0.0)
                    .rotate(2.0 * f32::PI * index as f32 / len as f32);
        }
        let cursor_delta = self.camera.screen_to_world(
            self.framebuffer_size,
            self.ctx.geng.window().cursor_position().map(|x| x as f32),
        ) - center;
        if cursor_delta.len() > self.config.brush_wheel.inner_radius {
            if let Some(item) = items
                .iter_mut()
                .filter(|item| vec2::dot(item.pos - center, cursor_delta) > 0.0)
                .min_by_key(|item| r32(vec2::skew(item.pos - center, cursor_delta).abs()))
            {
                item.hovered = true;
            }
        }
        Some(items.into_iter())
    }

    fn save(&mut self) {
        // TODO saved flag & warning
        ron::ser::to_writer_pretty(
            std::io::BufWriter::new(std::fs::File::create(&self.path).unwrap()),
            &self.game_state,
            default(),
        )
        .unwrap();
    }

    fn undo(&mut self) {
        if self.history.len() > 1 {
            if *self.game_state != self.history.pop().unwrap() {
                log::error!("DID YOU JUST CTRL-Z WHILE PAINTING?");
            }
            *self.game_state = self.history.last().unwrap().clone();
            self.level_mesh = self.ctx.renderer.level_mesh(&self.game_state);
        }
    }

    fn push_history_if_needed(&mut self) {
        if *self.game_state != *self.history.last().unwrap() {
            log::debug!("Pushed history");
            self.history.push(self.game_state.clone());
        }
    }

    fn assign_index(&mut self, index: i32) {
        let cell = self.screen_to_tile(self.ctx.geng.window().cursor_position());
        if let Some(entity) = self
            .game_state
            .entities
            .iter_mut()
            .find(|entity| entity.pos.cell == cell)
        {
            entity.index = Some(index);
        }
        self.push_history_if_needed();
    }
}

impl Drop for State<'_> {
    fn drop(&mut self) {
        self.save();
    }
}

impl State<'_> {
    pub async fn run(mut self, actx: &mut async_states::Context) {
        loop {
            match actx.wait().await {
                async_states::Event::Event(event) => {
                    if let std::ops::ControlFlow::Break(()) = self.handle_event(actx, event).await {
                        break;
                    }
                }
                async_states::Event::Update(delta_time) => self.update(delta_time),
                async_states::Event::Draw => self.draw(&mut actx.framebuffer()),
            }
        }
    }
    fn update(&mut self, delta_time: f64) {
        let _delta_time = delta_time as f32;
        if self.autosave_timer.elapsed().as_secs_f64() > self.config.autosave_timer {
            self.save();
            self.autosave_timer.reset();
        }
    }
    async fn handle_event(
        &mut self,
        actx: &mut async_states::Context,
        event: geng::Event,
    ) -> std::ops::ControlFlow<()> {
        let controls = &self.config.controls;
        self.camera_controls
            .handle_event(&mut self.camera, event.clone());
        match event {
            geng::Event::KeyDown { key }
                if self.ctx.assets.config.controls.escape.contains(&key) =>
            {
                return std::ops::ControlFlow::Break(());
            }
            geng::Event::KeyDown { key } if key == controls.grid => {
                self.show_grid = !self.show_grid;
            }
            geng::Event::KeyDown { key } if key == controls.toggle => {
                play::State::new(&self.ctx, self.game_state.clone())
                    .run(actx)
                    .await;
            }
            geng::Event::KeyDown { key } if key == controls.choose => {
                self.brush_wheel_pos = Some(self.camera.screen_to_world(
                    self.framebuffer_size,
                    self.ctx.geng.window().cursor_position().map(|x| x as f32),
                ));
            }
            geng::Event::KeyUp { key } if key == controls.choose => {
                let hovered_item = self
                    .brush_wheel()
                    .into_iter()
                    .flatten()
                    .find(|item| item.hovered);
                if let Some(item) = hovered_item {
                    self.brush = item.brush;
                }
                self.brush_wheel_pos = None;
            }
            geng::Event::KeyDown { key } if key == controls.pick => {
                if let Some(brush) = Brush::pick(
                    &self.game_state,
                    self.screen_to_tile(self.ctx.geng.window().cursor_position()),
                ) {
                    self.brush = brush;
                }
            }
            geng::Event::MouseDown { position, button } if button == controls.create => {
                self.create(position);
            }
            geng::Event::MouseDown { position, button } if button == controls.delete => {
                self.delete(position);
            }
            geng::Event::MouseUp { button, .. }
                if [controls.create, controls.delete].contains(&button) =>
            {
                self.push_history_if_needed();
            }
            geng::Event::MouseMove { position, .. } => {
                if self.ctx.geng.window().is_button_pressed(controls.create) {
                    self.create(position);
                } else if self.ctx.geng.window().is_button_pressed(controls.delete) {
                    self.delete(position);
                }
            }
            geng::Event::KeyDown { key } if key == controls.rotate => {
                let mut delta = 1;
                if self.ctx.geng.window().is_key_pressed(geng::Key::LShift) {
                    delta = -delta;
                }
                self.brush.angle = self.brush.angle.with_input(Input::from_sign(delta));
            }
            geng::Event::KeyDown { key: geng::Key::S }
                if self.ctx.geng.window().is_key_pressed(geng::Key::LCtrl) =>
            {
                self.save();
            }
            geng::Event::KeyDown { key: geng::Key::Z }
                if self.ctx.geng.window().is_key_pressed(geng::Key::LCtrl) =>
            {
                self.undo();
            }

            // TODO: macro?
            geng::Event::KeyDown {
                key: geng::Key::Num1,
            } => {
                self.assign_index(1);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num2,
            } => {
                self.assign_index(2);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num3,
            } => {
                self.assign_index(3);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num4,
            } => {
                self.assign_index(4);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num5,
            } => {
                self.assign_index(5);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num6,
            } => {
                self.assign_index(6);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num7,
            } => {
                self.assign_index(7);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num8,
            } => {
                self.assign_index(8);
            }
            geng::Event::KeyDown {
                key: geng::Key::Num9,
            } => {
                self.assign_index(9);
            }

            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.ctx.renderer.draw(
            framebuffer,
            &self.camera,
            history::Frame {
                current_state: &self.game_state,
                animation: None,
            },
            &self.level_mesh,
        );

        for entity in &self.game_state.entities {
            if let Some(index) = entity.index {
                self.ctx.geng.default_font().draw(
                    framebuffer,
                    &self.camera,
                    &index.to_string(),
                    vec2::splat(geng::TextAlign::CENTER),
                    mat3::translate(entity.pos.cell.map(|x| x as f32 + 0.5))
                        * mat3::scale_uniform(self.config.index_size),
                    self.config.index_color,
                );
            }
        }

        if self.show_grid {
            self.ctx
                .renderer
                .draw_grid(framebuffer, &self.camera, self.config.grid_color);
        }

        self.ctx.renderer.draw_tile(
            framebuffer,
            &self.camera,
            &self.brush.brush_type.tile_name(),
            Rgba::new(1.0, 1.0, 1.0, self.config.brush_preview_opacity),
            mat3::translate(
                self.screen_to_tile(self.ctx.geng.window().cursor_position())
                    .map(|x| x as f32),
            ) * mat3::rotate_around(vec2::splat(0.5), self.brush.rotation()),
        );
        self.ctx.renderer.draw_tile(
            framebuffer,
            &self.camera,
            "EditorSelect",
            Rgba::WHITE,
            mat3::translate(
                self.screen_to_tile(self.ctx.geng.window().cursor_position())
                    .map(|x| x as f32),
            ),
        );

        self.ctx.geng.default_font().draw(
            framebuffer,
            &self.camera,
            &self.title,
            vec2::splat(geng::TextAlign::LEFT),
            mat3::translate(self.game_state.bounding_box().top_left().map(|x| x as f32)),
            Rgba::WHITE,
        );

        if let Some(wheel) = self.brush_wheel() {
            let center = self.brush_wheel_pos.unwrap();
            let config = &self.config.brush_wheel;
            self.ctx.geng.draw2d().draw2d(
                framebuffer,
                &self.camera,
                &draw2d::Ellipse::circle_with_cut(
                    center,
                    config.inner_radius,
                    2.0 * config.radius - config.inner_radius,
                    config.color,
                ),
            );
            for item in wheel {
                self.ctx.renderer.draw_tile(
                    framebuffer,
                    &self.camera,
                    &item.brush.brush_type.tile_name(),
                    Rgba::WHITE,
                    mat3::translate(item.pos)
                        * mat3::scale_uniform(if item.hovered { 2.0 } else { 1.0 })
                        * mat3::rotate(item.brush.rotation())
                        * mat3::translate(vec2::splat(-0.5)),
                );
            }
        }
    }
}
