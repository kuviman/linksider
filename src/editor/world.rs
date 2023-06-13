use super::*;

#[derive(Deserialize)]
pub struct Controls {
    insert: geng::Key,
    delete: geng::Key,
    copy: geng::Key,
    paste: geng::Key,
    rename: geng::Key,
}

#[derive(Deserialize)]
pub struct Config {
    default_fov: f32,
    min_fov: f32,
    max_fov: f32,
    level_icon_size: f32,
    margin: f32,
    preview_texture_size: usize,
    controls: Controls,
}

struct Level {
    name: String,
    state: logicsider::Level,
    preview: ugli::Texture,
}

struct Group {
    name: String,
    levels: Vec<Level>,
}

impl Group {
    fn save_level_list(&self) {
        levels::save_level_names(
            &self.name,
            &self
                .levels
                .iter()
                .map(|level| level.name.as_str())
                .collect::<Vec<_>>(),
        );
    }
}

struct Selection {
    group: usize,
    level: usize,
}

pub struct State {
    ctx: Rc<Context>,
    framebuffer_size: vec2<f32>,
    groups: Vec<Group>,
    camera: geng::Camera2d,
    input: input::State,
    config: Rc<Config>,
    register: Option<logicsider::Level>,
    drag: Option<Selection>,
}

fn level_screen_pos(group_index: usize, level_index: usize) -> vec2<i32> {
    vec2(level_index as i32, -(group_index as i32))
}

impl State {
    fn clamp_camera(&mut self) {
        let aabb = Aabb2::points_bounding_box(self.groups.iter().enumerate().flat_map(
            |(group_index, group)| {
                group
                    .levels
                    .iter()
                    .enumerate()
                    .map(move |(level_index, _level)| level_screen_pos(group_index, level_index))
            },
        ))
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

    fn hovered_with_screen_pos(&self, screen_pos: vec2<f64>) -> Option<Selection> {
        let world_pos = self.camera.screen_to_world(
            self.ctx.geng.window().size().map(|x| x as f32),
            screen_pos.map(|x| x as f32),
        );
        self.hovered_with_world_pos(world_pos)
    }

    fn hovered_with_world_pos(&self, world_pos: vec2<f32>) -> Option<Selection> {
        let places = self
            .groups
            .iter()
            .enumerate()
            .flat_map(|(group_index, group)| {
                group
                    .levels
                    .iter()
                    .enumerate()
                    .map(move |(level_index, _level)| (group_index, level_index))
                    .chain([(group_index, group.levels.len())])
            })
            .chain([(self.groups.len(), 0)]);
        for (group_index, level_index) in places {
            if Aabb2::point(level_screen_pos(group_index, level_index))
                .extend_positive(vec2::splat(1))
                .map(|x| x as f32)
                .contains(world_pos)
            {
                return Some(Selection {
                    group: group_index,
                    level: level_index,
                });
            }
        }
        None
    }

    async fn insert_level(
        &mut self,
        actx: &mut async_states::Context,
        group_index: usize,
        level_index: usize,
        level: logicsider::Level,
    ) {
        if self.groups.get(group_index).is_none() {
            return;
        }
        let Some(name) = popup::prompt(&self.ctx, actx, "New level name", "").await else {
            return;
        };
        let group = &mut self.groups[group_index];
        level
            .save_to_file(levels::level_path(&group.name, &name))
            .unwrap();
        group.levels.insert(
            level_index,
            Level {
                name,
                preview: generate_preview(&self.ctx, &level),
                state: level,
            },
        );
        group.save_level_list();
    }

    fn save_group_list(&self) {
        levels::save_group_names(
            &self
                .groups
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
        );
    }

    fn reorder(&mut self, from: Selection, to: Selection) -> Option<()> {
        if self.groups.get(to.group).is_none() {
            return None;
        }
        let level = self
            .groups
            .get_mut(from.group)?
            .levels
            .try_remove(from.level)?;
        let level_name = level.name.clone();
        self.groups
            .get_mut(to.group)
            .unwrap()
            .levels
            .insert(to.level, level);
        self.groups[from.group].save_level_list();
        self.groups[to.group].save_level_list();
        if from.group != to.group {
            std::fs::rename(
                levels::level_path(&self.groups[from.group].name, &level_name),
                levels::level_path(&self.groups[to.group].name, &level_name),
            )
            .unwrap();
        }
        Some(())
    }

    async fn click_selection(&mut self, actx: &mut async_states::Context, selection: Selection) {
        if let Some(group) = self.groups.get_mut(selection.group) {
            if let Some(level) = group.levels.get_mut(selection.level) {
                editor::level::State::new(
                    &self.ctx,
                    format!("{}::{} (#{})", group.name, level.name, selection.level),
                    &mut level.state,
                    levels::level_path(&group.name, &level.name),
                )
                .run(actx)
                .await;
                level.preview = generate_preview(&self.ctx, &level.state);
            } else {
                self.insert_level(
                    actx,
                    selection.group,
                    selection.level,
                    logicsider::Level::empty(),
                )
                .await;
            }
        } else if let Some(name) = popup::prompt(&self.ctx, actx, "New group name", "").await {
            let group = Group {
                name,
                levels: Vec::new(),
            };
            std::fs::create_dir(levels::group_dir(&group.name)).unwrap();
            self.groups.push(group);
            self.save_group_list();
        }
    }

    fn start_drag(&mut self, screen_pos: vec2<f64>) {
        if let Some(selection) = self.hovered_with_screen_pos(screen_pos) {
            if self
                .groups
                .get(selection.group)
                .and_then(|group| group.levels.get(selection.level))
                .is_some()
            {
                self.drag = Some(selection);
            }
        }
    }
}

trait VecExt<T> {
    fn try_remove(&mut self, index: usize) -> Option<T>;
}

impl<T> VecExt<T> for Vec<T> {
    fn try_remove(&mut self, index: usize) -> Option<T> {
        if index < self.len() {
            Some(self.remove(index))
        } else {
            None
        }
    }
}

impl input::Context for State {
    fn input(&mut self) -> &mut input::State {
        &mut self.input
    }
    fn is_draggable(&self, screen_pos: vec2<f64>) -> bool {
        self.hovered_with_screen_pos(screen_pos).is_some()
    }
}

impl State {
    async fn run(mut self, actx: &mut async_states::Context) {
        loop {
            match actx.wait().await {
                async_states::Event::Event(event) => self.handle_event(actx, event).await,
                async_states::Event::Update(delta_time) => self.update(actx, delta_time).await,
                async_states::Event::Draw => self.draw(&mut actx.framebuffer()),
            }
        }
    }
    async fn update(&mut self, actx: &mut async_states::Context, delta_time: f64) {
        for event in input::Context::update(self, delta_time) {
            self.handle_input(actx, event).await;
        }
    }
    async fn handle_input(&mut self, actx: &mut async_states::Context, event: input::Event) {
        match event {
            input::Event::DragStart(pos) => {
                self.start_drag(pos);
            }
            input::Event::DragMove(_) => {}
            input::Event::DragEnd(pos) => {
                if let Some(from) = self.drag.take() {
                    if let Some(to) = self.hovered_with_screen_pos(pos) {
                        if self.ctx.geng.window().is_key_pressed(geng::Key::LCtrl) {
                            self.insert_level(
                                actx,
                                to.group,
                                to.level,
                                self.groups[from.group].levels[from.level].state.clone(),
                            )
                            .await;
                        } else {
                            self.reorder(from, to);
                        }
                    }
                }
            }
            input::Event::Click(pos) => {
                if let Some(selection) = self.hovered_with_screen_pos(pos) {
                    self.click_selection(actx, selection).await;
                }
            }
            input::Event::TransformView(transform) => {
                transform.apply(&mut self.camera, self.framebuffer_size);
                self.clamp_camera();
            }
        }
    }
    async fn handle_event(&mut self, actx: &mut async_states::Context, event: geng::Event) {
        for event in input::Context::handle_event(self, event.clone()) {
            self.handle_input(actx, event).await;
        }
        match event {
            geng::Event::KeyDown { key } => {
                if let Some(selection) =
                    self.hovered_with_screen_pos(self.ctx.geng.window().cursor_position())
                {
                    if self.config.controls.rename == key {
                        if let Some(group) = self.groups.get_mut(selection.group) {
                            if let Some(level) = group.levels.get_mut(selection.level) {
                                if let Some(new_name) =
                                    popup::prompt(&self.ctx, actx, "Rename level", &level.name)
                                        .await
                                {
                                    std::fs::rename(
                                        levels::level_path(&group.name, &level.name),
                                        levels::level_path(&group.name, &new_name),
                                    )
                                    .unwrap();
                                    level.name = new_name;
                                    group.save_level_list();
                                }
                            } else {
                                if let Some(new_name) =
                                    popup::prompt(&self.ctx, actx, "Rename group", &group.name)
                                        .await
                                {
                                    std::fs::rename(
                                        levels::group_dir(&group.name),
                                        levels::group_dir(&new_name),
                                    )
                                    .unwrap();
                                    group.name = new_name;
                                    self.save_group_list();
                                }
                            }
                        }
                    }
                    if self.config.controls.insert == key {
                        if self.groups.get(selection.group).is_some() {
                            self.insert_level(
                                actx,
                                selection.group,
                                selection.level,
                                logicsider::Level::empty(),
                            )
                            .await;
                        }
                    }
                    if self.config.controls.delete == key {
                        if let Some(group) = self.groups.get_mut(selection.group) {
                            if selection.level < group.levels.len() {
                                if popup::confirm(
                                    &self.ctx,
                                    actx,
                                    &format!(
                                        "Are you sure you want to delete level\n{}::{}",
                                        group.name, group.levels[selection.level].name,
                                    ),
                                )
                                .await
                                {
                                    let level = group.levels.remove(selection.level);
                                    std::fs::remove_file(levels::level_path(
                                        &group.name,
                                        &level.name,
                                    ))
                                    .unwrap();
                                    self.register = Some(level.state);
                                    group.save_level_list();
                                }
                            }
                        }
                    }
                    if self.config.controls.copy == key {
                        if let Some(level) = self
                            .groups
                            .get(selection.group)
                            .and_then(|group| group.levels.get(selection.level))
                        {
                            self.register = Some(level.state.clone());
                        }
                    }
                    if self.config.controls.paste == key {
                        if self.groups.get(selection.group).is_some() {
                            if let Some(state) = self.register.clone() {
                                self.insert_level(actx, selection.group, selection.level, state)
                                    .await;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.clamp_camera();
        self.ctx.renderer.draw_background(framebuffer, &self.camera);
        for (group_index, group) in self.groups.iter().enumerate() {
            for (level_index, level) in group.levels.iter().enumerate() {
                if let Some(drag) = &self.drag {
                    if drag.group == group_index && drag.level == level_index {
                        continue;
                    }
                }
                self.ctx.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::TexturedQuad::new(
                        Aabb2::point(
                            level_screen_pos(group_index, level_index).map(|x| x as f32 + 0.5),
                        )
                        .extend_symmetric(vec2::splat(self.config.level_icon_size / 2.0)),
                        &level.preview,
                    ),
                );
            }
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "Plus",
                Rgba::WHITE,
                mat3::translate(
                    level_screen_pos(group_index, group.levels.len()).map(|x| x as f32),
                ),
            );
        }
        self.ctx.renderer.draw_tile(
            framebuffer,
            &self.camera,
            "Plus",
            Rgba::WHITE,
            mat3::translate(level_screen_pos(self.groups.len(), 0).map(|x| x as f32)),
        );
        if let Some(drag) = &self.drag {
            let level = &self.groups[drag.group].levels[drag.level];
            self.ctx.geng.draw2d().draw2d(
                framebuffer,
                &self.camera,
                &draw2d::TexturedQuad::unit(&level.preview)
                    .scale_uniform(0.25)
                    .translate(self.camera.screen_to_world(
                        self.framebuffer_size,
                        self.ctx.geng.window().cursor_position().map(|x| x as f32),
                    )),
            );
        }
        if let Some(selection) =
            self.hovered_with_screen_pos(self.ctx.geng.window().cursor_position())
        {
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "EditorSelect",
                Rgba::WHITE,
                mat3::translate(
                    level_screen_pos(selection.group, selection.level).map(|x| x as f32),
                ),
            );
            let text = match self.groups.get(selection.group) {
                Some(group) => match group.levels.get(selection.level) {
                    Some(level) => format!("{}::{}", group.name, level.name),
                    None => "New level".to_owned(),
                },
                None => "New group".to_owned(),
            };
            self.ctx.geng.default_font().draw_with_outline(
                framebuffer,
                &self.camera,
                &text,
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(
                    level_screen_pos(selection.group, selection.level).map(|x| x as f32 + 0.5)
                        + vec2(0.0, 1.0),
                ),
                Rgba::WHITE,
                0.05,
                Rgba::BLACK,
            );
        }
    }
}

fn generate_preview(ctx: &Context, level: &logicsider::Level) -> ugli::Texture {
    let mut texture = ugli::Texture::new_uninitialized(
        ctx.geng.ugli(),
        vec2::splat(ctx.assets.config.editor.world.preview_texture_size),
    );
    texture.set_filter(ugli::Filter::Nearest);
    let bb = level.bounding_box().map(|x| x as f32);
    ctx.renderer.draw_level(
        &mut ugli::Framebuffer::new_color(
            ctx.geng.ugli(),
            ugli::ColorAttachment::Texture(&mut texture),
        ),
        &geng::Camera2d {
            fov: bb.height(),
            center: bb.center(),
            rotation: Angle::ZERO,
        },
        level,
        &ctx.renderer.level_mesh(level),
    );
    texture
}

impl State {
    pub async fn load(ctx: &Rc<Context>, actx: &mut async_states::Context) {
        let group_names = levels::load_group_names().await;
        let groups = future::join_all(group_names.into_iter().map(|group_name| async {
            let level_names = levels::load_level_names(&group_name).await;
            let levels = future::join_all(level_names.into_iter().map(|level_name| async {
                let level =
                    logicsider::Level::load_from_file(levels::level_path(&group_name, &level_name))
                        .await
                        .unwrap();
                Level {
                    name: level_name,
                    preview: generate_preview(ctx, &level),
                    state: level,
                }
            }))
            .await;
            Group {
                name: group_name,
                levels,
            }
        }))
        .await;
        let config = ctx.assets.config.editor.world.clone();
        let state = Self {
            framebuffer_size: vec2::splat(1.0),
            groups,
            camera: geng::Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: config.default_fov,
            },
            config,
            register: None,
            ctx: ctx.clone(),
            drag: None,
            input: input::State::new(ctx),
        };
        state.run(actx).await
    }
}
