use super::*;

#[derive(Deserialize)]
pub struct Controls {
    pub toggle: geng::Key,
    camera_drag: geng::MouseButton,
    create: geng::MouseButton,
    delete: geng::MouseButton,
    choose: geng::Key,
}

#[derive(Deserialize)]
struct BrushWheelConfig {
    radius: f32,
    inner_radius: f32,
    color: Rgba<f32>,
}

#[derive(Deserialize)]
pub struct Config {
    brush_preview_opacity: f32,
    brush_wheel: BrushWheelConfig,
    pub controls: Controls,
}

#[derive(Debug, Eq, PartialEq)]
enum Brush {
    Entity(String),
    Tile(Tile),
    Powerup(Effect),
}

impl Brush {
    fn tile_name(&self) -> String {
        match self {
            Brush::Entity(name) => name.clone(),
            Brush::Tile(tile) => format!("{tile:?}").to_lowercase(),
            Brush::Powerup(effect) => format!("{effect:?}Power"),
        }
    }
}

struct BrushWheelItem {
    brush: Brush,
    pos: vec2<f32>,
    hovered: bool,
}

pub struct State {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Rc<Assets>,
    game_state: GameState,
    camera: Camera2d,
    transition: Option<geng::state::Transition>,
    sound: Rc<sound::State>,
    renderer: Rc<Renderer>,
    level_mesh: renderer::LevelMesh,
    finish_callback: play::FinishCallback,
    camera_drag: Option<vec2<f64>>,
    brush: Brush,
    brush_wheel_pos: Option<vec2<f32>>,
}

impl State {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        renderer: &Rc<Renderer>,
        sound: &Rc<sound::State>,
        game_state: GameState,
        finish_callback: play::FinishCallback,
    ) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            framebuffer_size: vec2::splat(1.0),
            camera: Camera2d {
                center: game_state.center(),
                rotation: 0.0,
                fov: 250.0 / 16.0,
            },
            transition: None,
            sound: sound.clone(),
            renderer: renderer.clone(),
            level_mesh: renderer.level_mesh(&game_state),
            game_state,
            finish_callback,
            camera_drag: None,
            brush: Brush::Entity("Player".to_owned()),
            brush_wheel_pos: None,
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
        match &self.brush {
            Brush::Entity(name) => self.game_state.add_entity(
                name,
                &self.assets.logic_config.entities[name],
                Position {
                    cell,
                    angle: IntAngle::RIGHT,
                },
            ),
            Brush::Tile(tile) => {
                self.game_state.tiles.insert(cell, *tile);
                self.level_mesh = self.renderer.level_mesh(&self.game_state);
            }
            Brush::Powerup(effect) => {
                self.game_state.powerups.insert(Powerup {
                    id: self.game_state.id_gen.gen(),
                    pos: Position {
                        cell,
                        angle: IntAngle::DOWN,
                    },
                    effect: effect.clone(),
                });
            }
        }
    }

    fn delete(&mut self, screen_pos: vec2<f64>) {
        let tile = self.screen_to_tile(screen_pos);
        if self.game_state.tiles.remove(&tile).is_some() {
            self.level_mesh = self.renderer.level_mesh(&self.game_state);
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
            .assets
            .logic_config
            .entities
            .keys()
            .map(|name| Brush::Entity(name.clone()));
        let tiles = Tile::iter_variants().map(Brush::Tile);
        let powerups = Effect::iter_variants().map(Brush::Powerup);

        let mut items: Vec<BrushWheelItem> = itertools::chain![entities, tiles, powerups]
            .map(|brush| BrushWheelItem {
                brush,
                pos: vec2::ZERO,
                hovered: false,
            })
            .collect();
        let len = items.len();
        for (index, item) in items.iter_mut().enumerate() {
            item.pos = center
                + vec2(self.assets.config.editor.brush_wheel.radius, 0.0)
                    .rotate(2.0 * f32::PI * index as f32 / len as f32);
        }
        let cursor_delta = self.camera.screen_to_world(
            self.framebuffer_size,
            self.geng.window().cursor_position().map(|x| x as f32),
        ) - center;
        if cursor_delta.len() > self.assets.config.editor.brush_wheel.inner_radius {
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
}

impl geng::State for State {
    fn update(&mut self, delta_time: f64) {
        let _delta_time = delta_time as f32;
    }
    fn transition(&mut self) -> Option<geng::state::Transition> {
        self.transition.take()
    }
    fn handle_event(&mut self, event: geng::Event) {
        let controls = &self.assets.config.editor.controls;
        match event {
            geng::Event::KeyDown { key } if key == controls.toggle => {
                self.transition =
                    Some(geng::state::Transition::Switch(Box::new(play::State::new(
                        &self.geng,
                        &self.assets,
                        &self.renderer,
                        &self.sound,
                        self.game_state.clone(),
                        self.finish_callback.clone(),
                    ))));
            }
            geng::Event::KeyDown { key } if key == controls.choose => {
                self.brush_wheel_pos = Some(self.camera.screen_to_world(
                    self.framebuffer_size,
                    self.geng.window().cursor_position().map(|x| x as f32),
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
            geng::Event::MouseDown { position, button } if button == controls.create => {
                self.create(position);
            }
            geng::Event::MouseDown { position, button } if button == controls.delete => {
                self.delete(position);
            }
            geng::Event::MouseDown { position, button } if button == controls.camera_drag => {
                self.camera_drag = Some(position);
            }
            geng::Event::MouseUp { button, .. } if button == controls.camera_drag => {
                self.camera_drag = None;
            }
            geng::Event::MouseMove { position, .. } => {
                if self.geng.window().is_button_pressed(controls.create) {
                    self.create(position);
                } else if self.geng.window().is_button_pressed(controls.delete) {
                    self.delete(position);
                } else if let Some(drag) = &mut self.camera_drag {
                    let world_pos = |pos: vec2<f64>| -> vec2<f32> {
                        self.camera
                            .screen_to_world(self.framebuffer_size, pos.map(|x| x as f32))
                    };
                    let before = world_pos(*drag);
                    let now = world_pos(position);
                    self.camera.center += before - now;
                    *drag = position;
                }
            }
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.renderer.draw(
            framebuffer,
            &self.camera,
            history::Frame {
                current_state: &self.game_state,
                animation: None,
            },
            &self.level_mesh,
        );

        self.renderer.draw_tile(
            framebuffer,
            &self.camera,
            &self.brush.tile_name(),
            Rgba::new(
                1.0,
                1.0,
                1.0,
                self.assets.config.editor.brush_preview_opacity,
            ),
            mat3::translate(
                self.screen_to_tile(self.geng.window().cursor_position())
                    .map(|x| x as f32),
            ),
        );
        self.renderer.draw_tile(
            framebuffer,
            &self.camera,
            "EditorSelect",
            Rgba::WHITE,
            mat3::translate(
                self.screen_to_tile(self.geng.window().cursor_position())
                    .map(|x| x as f32),
            ),
        );

        if let Some(wheel) = self.brush_wheel() {
            let center = self.brush_wheel_pos.unwrap();
            let config = &self.assets.config.editor.brush_wheel;
            self.geng.draw2d().draw2d(
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
                self.renderer.draw_tile(
                    framebuffer,
                    &self.camera,
                    &item.brush.tile_name(),
                    Rgba::WHITE,
                    mat3::translate(item.pos)
                        * mat3::scale_uniform(if item.hovered { 2.0 } else { 1.0 })
                        * mat3::translate(vec2::splat(-0.5)),
                );
            }
        }
    }
}
