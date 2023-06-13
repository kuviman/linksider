use super::*;

#[derive(Deserialize)]
pub struct Controls {
    pub toggle: geng::Key,
    choose: geng::Key,
    pick: geng::Key,
    grid: geng::Key,
    rotate: geng::Key,
    reset_to_last_save: geng::Key,
}

#[derive(Deserialize)]
struct ToolWheelConfig {
    radius: f32,
    inner_radius: f32,
    color: Rgba<f32>,
}

#[derive(Deserialize)]
pub struct Config {
    default_fov: f32,
    min_fov: f32,
    max_fov: f32,
    ui_fov: f32,
    margin: f32,
    index_size: f32,
    index_color: Rgba<f32>,
    grid_color: Rgba<f32>,
    preview_opacity: f32,
    tool_wheel: ToolWheelConfig,
    autosave_timer: Option<f64>,
    warning_size: f32,
    warning_color: Rgba<f32>,
    pub controls: Controls,
}

#[derive(PartialEq, Eq)]
enum ToolType {
    Entity(String),
    SideEffect(Effect),
    Powerup(Effect),
    Goal,
    Eraser,
}

impl ToolType {
    fn delete_underneath(&self) -> bool {
        match self {
            Self::SideEffect(_) => false,
            Self::Eraser => true,
            _ => true,
        }
    }
    fn tile_name(&self) -> String {
        match self {
            Self::Entity(name) => name.clone(),
            Self::SideEffect(effect) => format!("{effect:?}Power"),
            Self::Powerup(effect) => format!("{effect:?}Power"),
            Self::Goal => "Goal".to_owned(),
            Self::Eraser => "Eraser".to_owned(),
        }
    }

    fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        ctx: &Context,
        camera: &impl geng::AbstractCamera2d,
        matrix: mat3<f32>,
    ) {
        // TypeScript is better
        match self {
            Self::SideEffect(_) => {
                let matrix = matrix * mat3::scale_uniform(0.8);
                ctx.renderer.draw_tile(
                    framebuffer,
                    camera,
                    "Player",
                    Rgba::WHITE,
                    matrix * mat3::translate(vec2::splat(-0.5)),
                );
                ctx.renderer.draw_tile(
                    framebuffer,
                    camera,
                    &self.tile_name(),
                    Rgba::WHITE,
                    matrix * mat3::translate(vec2(-0.5, 0.5)),
                );
            }
            Self::Powerup(_) => {
                ctx.renderer.draw_tile(
                    framebuffer,
                    camera,
                    &self.tile_name(),
                    Rgba::WHITE,
                    matrix * mat3::translate(vec2(-0.5, 0.0)),
                );
            }
            _ => {
                ctx.renderer.draw_tile(
                    framebuffer,
                    camera,
                    &self.tile_name(),
                    Rgba::WHITE,
                    matrix * mat3::translate(vec2::splat(-0.5)),
                );
            }
        }
    }

    fn show_preview(&self) -> bool {
        match self {
            Self::Eraser => false,
            _ => true,
        }
    }
}

struct Tool {
    angle: IntAngle,
    tool_type: ToolType,
}

impl Tool {
    fn rotation(&self) -> Angle<f32> {
        // TODO normalize angles in the codebase
        let angle = self.angle;
        match self.tool_type {
            ToolType::SideEffect(_) | ToolType::Powerup(_) => angle.rotate_counter_clockwise(),
            _ => angle,
        }
        .to_angle()
    }
    fn pick(level: &Level, cell: vec2<i32>) -> Option<Self> {
        if let Some(entity) = level.entities.iter().find(|entity| entity.pos.cell == cell) {
            return Some(Self {
                angle: entity.pos.angle,
                tool_type: ToolType::Entity(entity.identifier.clone()),
            });
        }
        if let Some(powerup) = level
            .powerups
            .iter()
            .find(|powerup| powerup.pos.cell == cell)
        {
            return Some(Self {
                angle: powerup.pos.angle,
                tool_type: ToolType::Powerup(powerup.effect.clone()),
            });
        }
        if let Some(goal) = level.goals.iter().find(|goal| goal.pos.cell == cell) {
            return Some(Self {
                angle: goal.pos.angle,
                tool_type: ToolType::Goal,
            });
        }
        None
    }
}

struct ToolWheelItem {
    tool: Tool,
    pos: vec2<f32>,
    hovered: bool,
}

pub struct State<'a> {
    ctx: Context,
    title: String,
    framebuffer_size: vec2<f32>,
    config: Rc<Config>,
    level: &'a mut Level,
    camera: Camera2d,
    ui_camera: Camera2d,
    level_mesh: renderer::LevelMesh,
    input: input::State,
    tool: Tool,
    tool_wheel_pos: Option<vec2<f32>>,
    path: std::path::PathBuf,
    history: Vec<Rc<Level>>,
    history_pos: usize,
    saved: Rc<Level>,
    autosave_timer: Timer,
    show_grid: bool,
    dragged_entity: Option<usize>,
    drag_pos: vec2<f64>,
}

impl<'a> State<'a> {
    pub fn new(
        ctx: &Context,
        title: String,
        level: &'a mut Level,
        path: impl AsRef<std::path::Path>,
    ) -> Self {
        let path = path.as_ref();
        let config = ctx.assets.config.editor.level.clone();
        let saved = Rc::new(level.clone());
        Self {
            title,
            autosave_timer: Timer::new(),
            path: path.to_owned(),
            framebuffer_size: vec2::splat(1.0),
            drag_pos: vec2::ZERO,
            camera: Camera2d {
                center: Aabb2::points_bounding_box(
                    level.entities.iter().map(|entity| entity.pos.cell),
                )
                .unwrap_or(Aabb2::ZERO)
                .extend_positive(vec2::splat(1))
                .map(|x| x as f32)
                .center(),
                rotation: Angle::ZERO,
                fov: config.default_fov,
            },
            ui_camera: Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: config.ui_fov,
            },
            config,
            level_mesh: ctx.renderer.level_mesh(&level),
            tool: Tool {
                angle: IntAngle::RIGHT,
                tool_type: ToolType::Entity("Player".to_owned()),
            },
            tool_wheel_pos: None,
            history: vec![saved.clone()],
            history_pos: 0,
            saved,
            level,
            show_grid: true,
            ctx: ctx.clone(),
            dragged_entity: None,
            input: input::State::new(ctx),
        }
    }

    fn clamp_camera(&mut self) {
        let aabb =
            Aabb2::points_bounding_box(self.level.entities.iter().map(|entity| entity.pos.cell))
                .unwrap()
                .extend_positive(vec2::splat(1))
                .map(|x| x as f32)
                .extend_uniform(self.config.margin);
        self.camera.center = self.camera.center.clamp_aabb({
            let mut aabb = aabb.extend_symmetric(
                -vec2(self.framebuffer_size.aspect(), 1.0) * self.camera.fov / 2.0,
            );
            if aabb.min.x > aabb.max.x {
                let center = (aabb.min.x + aabb.max.x) / 2.0;
                aabb.min.x = center;
                aabb.max.x = center;
            }
            if aabb.min.y > aabb.max.y {
                let center = (aabb.min.y + aabb.max.y) / 2.0;
                aabb.min.y = center;
                aabb.max.y = center;
            }
            aabb
        });
        self.camera.rotation = Angle::ZERO;
        self.camera.fov = self
            .camera
            .fov
            .clamp(self.config.min_fov, self.config.max_fov);
    }

    fn screen_to_cell(&self, screen_pos: vec2<f64>) -> vec2<i32> {
        let world_pos = self
            .camera
            .screen_to_world(self.framebuffer_size, screen_pos.map(|x| x as f32));
        world_pos.map(|x| x.floor() as i32)
    }

    fn use_tool(&mut self, screen_pos: vec2<f64>) {
        let cell = self.screen_to_cell(screen_pos);
        if self.tool.tool_type == ToolType::Entity("Player".to_owned()) {
            if let Some(entity) = self
                .level
                .entities
                .iter()
                .find(|entity| entity.pos.cell == cell)
            {
                if entity.identifier == "Player" {
                    self.assign_index(None);
                    return;
                } else {
                    self.delete(screen_pos);
                }
            }
        } else if self.tool.tool_type.delete_underneath() {
            self.delete(screen_pos);
        }
        match &self.tool.tool_type {
            ToolType::Entity(name) => self.level.entities.push(logicsider::level::Entity {
                identifier: name.to_owned(),
                index: None,
                pos: Position {
                    cell,
                    angle: self.tool.angle,
                },
                sides: default(),
            }),
            ToolType::SideEffect(effect) => {
                if let Some(entity) = self
                    .level
                    .entities
                    .iter_mut()
                    .find(|entity| entity.pos.cell == cell)
                {
                    entity.side_at_angle_mut(self.tool.angle).effect = Some(effect.clone());
                }
            }
            ToolType::Powerup(effect) => {
                self.level.powerups.push(logicsider::level::Powerup {
                    pos: Position {
                        cell,
                        angle: self.tool.angle,
                    },
                    effect: effect.clone(),
                });
            }
            ToolType::Goal => self.level.goals.push(logicsider::level::Goal {
                pos: Position {
                    cell,
                    angle: self.tool.angle,
                },
            }),
            ToolType::Eraser => {}
        }
        self.level_mesh = self.ctx.renderer.level_mesh(self.level);
    }

    fn delete(&mut self, screen_pos: vec2<f64>) {
        let cell = self.screen_to_cell(screen_pos);
        self.level.entities.retain(|entity| entity.pos.cell != cell);
        self.level.powerups.retain(|entity| entity.pos.cell != cell);
        self.level.goals.retain(|entity| entity.pos.cell != cell);
        self.level_mesh = self.ctx.renderer.level_mesh(self.level);
    }

    fn open_wheel(&mut self) {
        self.tool_wheel_pos = Some(
            self.ui_camera
                .screen_to_world(self.framebuffer_size, self.drag_pos.map(|x| x as f32)),
        );
    }

    fn close_wheel(&mut self) {
        if self.tool_wheel_pos.is_none() {
            return;
        }
        let hovered_item = self
            .tool_wheel()
            .into_iter()
            .flatten()
            .find(|item| item.hovered);
        if let Some(item) = hovered_item {
            self.tool = item.tool;
        }
        self.tool_wheel_pos = None;
    }

    fn tool_wheel(&self) -> Option<impl Iterator<Item = ToolWheelItem> + '_> {
        let center = self.tool_wheel_pos?;
        let entities = self
            .ctx
            .assets
            .logic_config
            .entities
            .keys()
            .map(|name| ToolType::Entity(name.clone()))
            .map(|tool_type| Tool {
                angle: IntAngle::RIGHT,
                tool_type,
            });
        let powerups = Effect::iter_variants()
            .map(ToolType::Powerup)
            .map(|tool_type| Tool {
                angle: IntAngle::DOWN,
                tool_type,
            });
        let side_effects = Effect::iter_variants()
            .map(ToolType::SideEffect)
            .map(|tool_type| Tool {
                angle: IntAngle::DOWN,
                tool_type,
            });
        let goal = Tool {
            angle: IntAngle::RIGHT,
            tool_type: ToolType::Goal,
        };
        let eraser = Tool {
            angle: IntAngle::RIGHT,
            tool_type: ToolType::Eraser,
        };

        let mut items: Vec<ToolWheelItem> =
            itertools::chain![entities, powerups, side_effects, [goal, eraser]]
                .map(|tool| ToolWheelItem {
                    tool,
                    pos: vec2::ZERO,
                    hovered: false,
                })
                .collect();
        let len = items.len();
        for (index, item) in items.iter_mut().enumerate() {
            item.pos = center
                + vec2(self.config.tool_wheel.radius, 0.0)
                    .rotate(Angle::from_degrees(360.0 * index as f32 / len as f32));
        }
        let cursor_delta = self
            .ui_camera
            .screen_to_world(self.framebuffer_size, self.drag_pos.map(|x| x as f32))
            - center;
        if cursor_delta.len() > self.config.tool_wheel.inner_radius {
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
        log::debug!("Saving");
        if *self.history[self.history_pos] != *self.level {
            log::error!("History had incorrect data wtf");
            self.history[self.history_pos] = Rc::new(self.level.clone());
        }
        self.saved = self.history[self.history_pos].clone();
        self.saved.save_to_file(&self.path).unwrap();
    }

    fn saved(&self) -> bool {
        Rc::ptr_eq(&self.saved, &self.history[self.history_pos])
    }

    fn autosave_if_enabled(&mut self) {
        if self.config.autosave_timer.is_some() && !self.saved() {
            self.save();
        }
    }

    fn reset_to_last_save(&mut self) {
        *self.level = self.saved.deref().clone();
    }

    fn change_history_pos(&mut self, delta: isize) {
        let new_pos = self.history_pos as isize + delta;
        if new_pos < 0 || new_pos >= self.history.len() as isize {
            return;
        }
        let new_pos = new_pos as usize;
        self.history_pos = new_pos;
        *self.level = self.history[self.history_pos].deref().clone();
        self.level_mesh = self.ctx.renderer.level_mesh(&self.level);
    }

    fn undo(&mut self) {
        self.change_history_pos(-1);
    }

    fn redo(&mut self) {
        self.change_history_pos(1);
    }

    fn push_history_if_needed(&mut self) {
        if *self.level != *self.history[self.history_pos] {
            self.history_pos += 1;
            self.history.truncate(self.history_pos);
            self.history.push(Rc::new(self.level.clone()));
            log::debug!("Pushed history");
        }
    }

    fn assign_index(&mut self, index: Option<i32>) {
        let cell = self.screen_to_cell(self.ctx.geng.window().cursor_position());
        if let Some(entity) = self
            .level
            .entities
            .iter_mut()
            .find(|entity| entity.pos.cell == cell)
        {
            if entity.identifier == "Player" {
                let index = match index {
                    Some(index) => index,
                    None => match entity.index {
                        Some(index) => index % 9 + 1,
                        None => 1,
                    },
                };
                entity.index = Some(index);
            }
        }
        self.push_history_if_needed();
    }
}

impl State<'_> {
    pub async fn run(mut self, actx: &mut async_states::Context) {
        loop {
            match actx.wait().await {
                async_states::Event::Event(event) => {
                    if let std::ops::ControlFlow::Break(()) = self.handle_event(actx, event).await {
                        self.autosave_if_enabled();
                        if self.saved() {
                            break;
                        }
                        if popup::confirm(
                            &self.ctx,
                            actx,
                            "You have unsaved changes\nAre you sure you want yo exit?",
                        )
                        .await
                        {
                            self.reset_to_last_save();
                            break;
                        }
                    }
                }
                async_states::Event::Update(delta_time) => self.update(delta_time),
                async_states::Event::Draw => self.draw(&mut actx.framebuffer()),
            }
        }
    }
    fn update(&mut self, delta_time: f64) {
        for event in input::Context::update(self, delta_time) {
            self.handle_input(event);
        }

        let _delta_time = delta_time as f32;
        if let Some(autosave_timer) = self.config.autosave_timer {
            if self.autosave_timer.elapsed().as_secs_f64() > autosave_timer {
                self.autosave_if_enabled();
                self.autosave_timer.reset();
            }
        }
    }

    fn handle_input(&mut self, event: input::Event) {
        match event {
            input::Event::DragStart(position) => {
                self.dragged_entity = self
                    .level
                    .entities
                    .iter()
                    .position(|entity| entity.pos.cell == self.screen_to_cell(position));
                self.drag_pos = position;
                if self.dragged_entity.is_none() {
                    self.open_wheel();
                }
            }
            input::Event::DragMove(position) => {
                self.drag_pos = position;
            }
            input::Event::DragEnd(position) => {
                self.close_wheel();
                if let Some(index) = self.dragged_entity.take() {
                    self.level.entities[index].pos.cell = self.screen_to_cell(position);
                    self.level_mesh = self.ctx.renderer.level_mesh(self.level);
                    self.push_history_if_needed();
                }
            }
            input::Event::Click(position) => {
                self.use_tool(position);
                self.push_history_if_needed();
            }
            input::Event::TransformView(transform) => {
                transform.apply(&mut self.camera, self.framebuffer_size);
                self.clamp_camera();
            }
        }
    }
    async fn handle_event(
        &mut self,
        actx: &mut async_states::Context,
        event: geng::Event,
    ) -> std::ops::ControlFlow<()> {
        for event in input::Context::handle_event(self, event.clone()) {
            self.handle_input(event);
        }
        let controls = &self.config.controls;
        match event {
            geng::Event::MouseMove { position, .. } => {
                self.drag_pos = position;
            }
            geng::Event::KeyDown { key }
                if self.ctx.assets.config.controls.escape.contains(&key) =>
            {
                return std::ops::ControlFlow::Break(());
            }
            geng::Event::KeyDown { key } if key == controls.grid => {
                self.show_grid = !self.show_grid;
            }
            geng::Event::KeyDown { key } if key == controls.reset_to_last_save => {
                self.reset_to_last_save();
            }
            geng::Event::KeyDown { key } if key == controls.toggle => {
                play::State::new(&self.ctx, &self.level).run(actx).await;
            }
            geng::Event::KeyDown { key } if key == controls.choose => {
                self.open_wheel();
            }
            geng::Event::KeyUp { key } if key == controls.choose => {
                self.close_wheel();
            }
            geng::Event::KeyDown { key } if key == controls.pick => {
                if let Some(tool) = Tool::pick(
                    &self.level,
                    self.screen_to_cell(self.ctx.geng.window().cursor_position()),
                ) {
                    self.tool = tool;
                }
            }
            geng::Event::KeyDown { key } if key == controls.rotate => {
                let mut delta = 1;
                if self.ctx.geng.window().is_key_pressed(geng::Key::LShift) {
                    delta = -delta;
                }
                self.tool.angle = self.tool.angle.with_input(Input::from_sign(delta));
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
            geng::Event::KeyDown { key: geng::Key::Y }
                if self.ctx.geng.window().is_key_pressed(geng::Key::LCtrl) =>
            {
                self.redo();
            }

            // TODO: macro?
            geng::Event::KeyDown {
                key: geng::Key::Num1,
            } => {
                self.assign_index(Some(1));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num2,
            } => {
                self.assign_index(Some(2));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num3,
            } => {
                self.assign_index(Some(3));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num4,
            } => {
                self.assign_index(Some(4));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num5,
            } => {
                self.assign_index(Some(5));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num6,
            } => {
                self.assign_index(Some(6));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num7,
            } => {
                self.assign_index(Some(7));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num8,
            } => {
                self.assign_index(Some(8));
            }
            geng::Event::KeyDown {
                key: geng::Key::Num9,
            } => {
                self.assign_index(Some(9));
            }

            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.clamp_camera();
        self.ctx
            .renderer
            .draw_level(framebuffer, &self.camera, &self.level, &self.level_mesh);

        for entity in &self.level.entities {
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

        if self.tool_wheel_pos.is_none() {
            if self.tool.tool_type.show_preview() {
                self.ctx.renderer.draw_tile(
                    framebuffer,
                    &self.camera,
                    &self.tool.tool_type.tile_name(),
                    Rgba::new(1.0, 1.0, 1.0, self.config.preview_opacity),
                    mat3::translate(
                        self.screen_to_cell(self.ctx.geng.window().cursor_position())
                            .map(|x| x as f32),
                    ) * mat3::rotate_around(vec2::splat(0.5), self.tool.rotation()),
                );
            }
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "EditorSelect",
                Rgba::WHITE,
                mat3::translate(
                    self.screen_to_cell(self.ctx.geng.window().cursor_position())
                        .map(|x| x as f32),
                ),
            );
        }

        if let Some(index) = self.dragged_entity {
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                &self.level.entities[index].identifier,
                Rgba::WHITE,
                mat3::translate(
                    self.camera
                        .screen_to_world(self.framebuffer_size, self.drag_pos.map(|x| x as f32)),
                ) * mat3::scale_uniform(0.5)
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }

        self.ctx.geng.default_font().draw(
            framebuffer,
            &self.camera,
            &self.title,
            vec2::splat(geng::TextAlign::LEFT),
            mat3::translate(self.level.bounding_box().top_left().map(|x| x as f32)),
            Rgba::WHITE,
        );

        if let Some(wheel) = self.tool_wheel() {
            let center = self.tool_wheel_pos.unwrap();
            let config = &self.config.tool_wheel;
            self.ctx.geng.draw2d().draw2d(
                framebuffer,
                &self.ui_camera,
                &draw2d::Ellipse::circle_with_cut(
                    center,
                    config.inner_radius,
                    2.0 * config.radius - config.inner_radius,
                    config.color,
                ),
            );
            for item in wheel {
                item.tool.tool_type.draw(
                    framebuffer,
                    &self.ctx,
                    &self.ui_camera,
                    mat3::translate(item.pos)
                        * mat3::scale_uniform(if item.hovered { 2.0 } else { 1.0 })
                        * mat3::rotate(item.tool.rotation()),
                );
            }
        }

        if !self.saved() {
            self.ctx.geng.default_font().draw(
                framebuffer,
                &geng::PixelPerfectCamera,
                "You have unsaved changes",
                vec2::splat(geng::TextAlign::LEFT),
                mat3::scale_uniform(self.config.warning_size),
                self.config.warning_color,
            );
        }
    }
}

impl input::Context for State<'_> {
    fn input(&mut self) -> &mut input::State {
        &mut self.input
    }

    fn is_draggable(&self, _screen_pos: vec2<f64>) -> bool {
        true
    }
}
