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
    fov: f32,
    level_icon_size: f32,
    margin: f32,
    preview_texture_size: usize,
    min_drag_distance: f64,
    max_click_time: f64,
    controls: Controls,
}

struct Level {
    name: String,
    state: logicsider::Level,
    preview: ugli::Texture,
}

fn level_path(group_name: &str, level_name: &str) -> std::path::PathBuf {
    group_dir(group_name).join(format!("{level_name}.ron"))
}

struct Group {
    name: String,
    levels: Vec<Level>,
}

impl Group {
    fn save_level_list(&self) {
        let path = group_dir(&self.name).join("list.ron");
        let writer = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        ron::ser::to_writer_pretty(
            writer,
            &self
                .levels
                .iter()
                .map(|level| &level.name)
                .collect::<Vec<_>>(),
            default(),
        )
        .unwrap();
    }
}

fn group_dir(group_name: &str) -> std::path::PathBuf {
    run_dir().join("levels").join(group_name)
}

fn groups_list_file() -> std::path::PathBuf {
    run_dir().join("levels").join("groups.ron")
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
    camera_controls: CameraControls,
    config: Rc<Config>,
    register: Option<logicsider::Level>,
    click_start: Option<(vec2<f64>, Timer)>,
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
    }

    fn hovered(&self, screen_pos: vec2<f64>) -> Option<Selection> {
        let world_pos = self.camera.screen_to_world(
            self.ctx.geng.window().size().map(|x| x as f32),
            screen_pos.map(|x| x as f32),
        );
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
        level.save_to_file(level_path(&group.name, &name)).unwrap();
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
        ron::ser::to_writer_pretty(
            std::io::BufWriter::new(std::fs::File::create(groups_list_file()).unwrap()),
            &self
                .groups
                .iter()
                .map(|group| &group.name)
                .collect::<Vec<_>>(),
            default(),
        )
        .unwrap();
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
                level_path(&self.groups[from.group].name, &level_name),
                level_path(&self.groups[to.group].name, &level_name),
            )
            .unwrap();
        }
        Some(())
    }

    async fn click_selection(&mut self, actx: &mut async_states::Context, selection: Selection) {
        if let Some(group) = self.groups.get_mut(selection.group) {
            if let Some(level) = group.levels.get_mut(selection.level) {
                let level_path = level_path(&group.name, &level.name);
                editor::level::State::new(
                    &self.ctx,
                    format!("{}::{} (#{})", group.name, level.name, selection.level),
                    &mut level.state,
                    level_path,
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
            std::fs::create_dir(group_dir(&group.name)).unwrap();
            self.groups.push(group);
            self.save_group_list();
        }
    }

    fn start_drag(&mut self, position: vec2<f64>) {
        if let Some(selection) = self.hovered(position) {
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

impl State {
    async fn run(mut self, actx: &mut async_states::Context) {
        loop {
            match actx.wait().await {
                async_states::Event::Event(event) => self.handle_event(actx, event).await,
                async_states::Event::Update(delta_time) => self.update(delta_time),
                async_states::Event::Draw => self.draw(&mut actx.framebuffer()),
            }
        }
    }
    fn update(&mut self, _delta_time: f64) {
        if let Some((start, timer)) = &self.click_start {
            if timer.elapsed().as_secs_f64() > self.config.max_click_time {
                self.start_drag(*start);
            }
        }
    }
    async fn handle_event(&mut self, actx: &mut async_states::Context, event: geng::Event) {
        if self
            .camera_controls
            .handle_event(&mut self.camera, event.clone())
        {
            return;
        }
        match event {
            geng::Event::KeyDown { key } => {
                if let Some(selection) = self.hovered(self.ctx.geng.window().cursor_position()) {
                    if self.config.controls.rename == key {
                        if let Some(group) = self.groups.get_mut(selection.group) {
                            if let Some(level) = group.levels.get_mut(selection.level) {
                                if let Some(new_name) =
                                    popup::prompt(&self.ctx, actx, "Rename level", &level.name)
                                        .await
                                {
                                    std::fs::rename(
                                        level_path(&group.name, &level.name),
                                        level_path(&group.name, &new_name),
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
                                    std::fs::rename(group_dir(&group.name), group_dir(&new_name))
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
                                self.register = Some(group.levels.remove(selection.level).state);
                                group.save_level_list();
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
            geng::Event::MouseDown {
                position,
                button: _,
            } => {
                self.click_start = Some((position, Timer::new()));
            }
            geng::Event::MouseMove { position, .. } => {
                if let Some((start, _)) = self.click_start {
                    if (start - position).len() > self.config.min_drag_distance {
                        self.start_drag(start);
                    }
                }
            }
            geng::Event::MouseUp {
                position,
                button: _,
            } => {
                let click_start = self.click_start.take();
                let drag = self.drag.take();
                if let Some(selection) = self.hovered(position) {
                    if let Some(drag) = drag {
                        if self.ctx.geng.window().is_key_pressed(geng::Key::LCtrl) {
                            self.insert_level(
                                actx,
                                selection.group,
                                selection.level,
                                self.groups[drag.group].levels[drag.level].state.clone(),
                            )
                            .await;
                        } else {
                            self.reorder(drag, selection);
                        }
                    } else if let Some((_, timer)) = click_start {
                        if timer.elapsed().as_secs_f64() < self.config.max_click_time {
                            self.click_selection(actx, selection).await;
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
        if let Some(selection) = self.hovered(self.ctx.geng.window().cursor_position()) {
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
        let group_names: Vec<String> = file::load_detect(groups_list_file()).await.unwrap();
        let groups = future::join_all(group_names.into_iter().map(|group_name| async {
            let list_path = group_dir(&group_name).join("list.ron");
            let level_names: Vec<String> = if list_path.is_file() {
                file::load_detect(list_path).await.unwrap()
            } else {
                // TODO remove
                let level_count: usize =
                    file::load_string(group_dir(&group_name).join("count.txt"))
                        .await
                        .unwrap()
                        .trim()
                        .parse()
                        .unwrap();
                (0..level_count).map(|x| x.to_string()).collect()
            };
            let levels = future::join_all(level_names.into_iter().map(|level_name| async {
                let level = logicsider::Level::load_from_file(level_path(&group_name, &level_name))
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
                fov: config.fov,
            },
            camera_controls: CameraControls::new(&ctx.geng, &ctx.assets.config.camera_controls),
            config,
            register: None,
            ctx: ctx.clone(),
            drag: None,
            click_start: None,
        };
        state.run(actx).await
    }
}
