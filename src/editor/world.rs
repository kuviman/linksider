use super::*;

#[derive(Deserialize)]
pub struct Controls {
    insert: geng::Key,
    delete: geng::Key,
    copy: geng::Key,
    paste: geng::Key,
}

#[derive(Deserialize)]
pub struct Config {
    fov: f32,
    level_icon_size: f32,
    margin: f32,
    preview_texture_size: usize,
    controls: Controls,
}

struct Level {
    name: String,
    state: GameState,
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

    fn generate_new_level_name(&self) -> String {
        // TODO better
        use geng::prelude::rand::distributions::DistString;
        rand::distributions::Alphanumeric.sample_string(&mut thread_rng(), 10)
    }
}

fn generate_new_group_name() -> String {
    // TODO better
    use geng::prelude::rand::distributions::DistString;
    rand::distributions::Alphanumeric.sample_string(&mut thread_rng(), 10)
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
    register: Option<GameState>,
}

impl State {
    fn clamp_camera(&mut self) {
        let aabb = Aabb2::ZERO
            .extend_positive(vec2(
                self.groups
                    .iter()
                    .map(|group| group.levels.len())
                    .max()
                    .unwrap_or(0),
                self.groups.len(),
            ))
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
            if Aabb2::point(vec2(level_index, group_index))
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

    fn insert_level(&mut self, group_index: usize, level_index: usize, game_state: GameState) {
        let group = &mut self.groups[group_index];
        let name = group.generate_new_level_name();
        ron::ser::to_writer_pretty(
            std::io::BufWriter::new(
                std::fs::File::create(&level_path(&group.name, &name)).unwrap(),
            ),
            &game_state,
            default(),
        )
        .unwrap();
        group.levels.insert(
            level_index,
            Level {
                name,
                preview: generate_preview(&self.ctx, &game_state),
                state: game_state,
            },
        );
        group.save_level_list();
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
    fn update(&mut self, _delta_time: f64) {}
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
                    if self.config.controls.insert == key {
                        if self.groups.get(selection.group).is_some() {
                            self.insert_level(selection.group, selection.level, GameState::empty());
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
                                self.insert_level(selection.group, selection.level, state);
                            }
                        }
                    }
                }
            }
            geng::Event::MouseDown {
                position,
                button: _,
            } => {
                if let Some(selection) = self.hovered(position) {
                    if let Some(group) = self.groups.get_mut(selection.group) {
                        if let Some(level) = group.levels.get_mut(selection.level) {
                            let level_path = level_path(&group.name, &level.name);
                            editor::level::State::new(&self.ctx, &mut level.state, level_path)
                                .run(actx)
                                .await;
                            level.preview = generate_preview(&self.ctx, &level.state);
                        } else {
                            self.insert_level(selection.group, selection.level, GameState::empty());
                        }
                    } else {
                        let group = Group {
                            name: generate_new_group_name(),
                            levels: Vec::new(),
                        };
                        std::fs::create_dir(group_dir(&group.name)).unwrap();
                        self.groups.push(group);
                        ron::ser::to_writer_pretty(
                            std::io::BufWriter::new(
                                std::fs::File::create(groups_list_file()).unwrap(),
                            ),
                            &self
                                .groups
                                .iter()
                                .map(|group| &group.name)
                                .collect::<Vec<_>>(),
                            default(),
                        )
                        .unwrap();
                        return;
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
                self.ctx.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::TexturedQuad::new(
                        Aabb2::point(vec2(level_index, group_index).map(|x| x as f32 + 0.5))
                            .extend_symmetric(vec2::splat(self.config.level_icon_size / 2.0)),
                        &level.preview,
                    ),
                )
            }
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "Plus",
                Rgba::WHITE,
                mat3::translate(vec2(group.levels.len(), group_index).map(|x| x as f32)),
            );
        }
        self.ctx.renderer.draw_tile(
            framebuffer,
            &self.camera,
            "Plus",
            Rgba::WHITE,
            mat3::translate(vec2(0, self.groups.len()).map(|x| x as f32)),
        );
        if let Some(selection) = self.hovered(self.ctx.geng.window().cursor_position()) {
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "EditorSelect",
                Rgba::WHITE,
                mat3::translate(vec2(selection.level as f32, selection.group as f32)),
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
                mat3::translate(vec2(
                    selection.level as f32 + 0.5,
                    selection.group as f32 + 1.5,
                )),
                Rgba::WHITE,
                0.05,
                Rgba::BLACK,
            );
        }
    }
}

fn generate_preview(ctx: &Context, game_state: &GameState) -> ugli::Texture {
    let mut texture = ugli::Texture::new_uninitialized(
        ctx.geng.ugli(),
        vec2::splat(ctx.assets.config.editor.world.preview_texture_size),
    );
    texture.set_filter(ugli::Filter::Nearest);
    let bb = game_state.bounding_box().map(|x| x as f32);
    ctx.renderer.draw(
        &mut ugli::Framebuffer::new_color(
            ctx.geng.ugli(),
            ugli::ColorAttachment::Texture(&mut texture),
        ),
        &geng::Camera2d {
            fov: bb.height(),
            center: bb.center(),
            rotation: 0.0,
        },
        history::Frame {
            current_state: &game_state,
            animation: None,
        },
        &ctx.renderer.level_mesh(&game_state),
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
                let game_state: GameState = file::load_detect(level_path(&group_name, &level_name))
                    .await
                    .unwrap();
                Level {
                    name: level_name,
                    preview: generate_preview(ctx, &game_state),
                    state: game_state,
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
            groups: groups,
            camera: geng::Camera2d {
                center: vec2::ZERO,
                rotation: 0.0,
                fov: config.fov,
            },
            camera_controls: CameraControls::new(&ctx.geng, &ctx.assets.config.camera_controls),
            config,
            register: None,
            ctx: ctx.clone(),
        };
        state.run(actx).await
    }
}
