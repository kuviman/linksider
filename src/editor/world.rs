use super::*;

#[derive(Deserialize)]
pub struct Config {
    fov: f32,
    level_icon_size: f32,
    margin: f32,
    preview_texture_size: usize,
}

struct Level {
    name: String,
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
    run_dir().join("assets").join(group_name)
}

fn groups_list_file() -> std::path::PathBuf {
    run_dir().join("levels").join("groups.ron")
}

struct Selection {
    group: usize,
    level: usize,
}

pub struct State {
    geng: Geng,
    assets: Rc<Assets>,
    sound: Rc<sound::State>,
    renderer: Rc<Renderer>,
    framebuffer_size: vec2<f32>,
    groups: Vec<Group>,
    camera: geng::Camera2d,
    camera_controls: CameraControls,
    config: Rc<Config>,
    transition: Option<geng::state::Transition>,
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
            self.geng.window().size().map(|x| x as f32),
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
}

impl geng::State for State {
    fn transition(&mut self) -> Option<geng::state::Transition> {
        self.transition.take()
    }
    fn handle_event(&mut self, event: geng::Event) {
        if self
            .camera_controls
            .handle_event(&mut self.camera, event.clone())
        {
            return;
        }
        match event {
            geng::Event::MouseDown {
                position,
                button: _,
            } => {
                if let Some(selection) = self.hovered(position) {
                    if self.groups.get(selection.group).is_none() {
                        let group = Group {
                            name: format!("Group{}", selection.group),
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
                    }
                    let group = &mut self.groups[selection.group];
                    if let Some(level) = group.levels.get(selection.level) {
                        let level_path = level_path(&group.name, &level.name);
                        self.transition = Some(geng::state::Transition::Switch(Box::new(
                            editor::level::State::load(
                                &self.geng,
                                &self.assets,
                                &self.sound,
                                &self.renderer,
                                level_path,
                            ),
                        )));
                    } else {
                        let name = format!("Level{}", selection.level);
                        let game_state = GameState::empty();
                        ron::ser::to_writer_pretty(
                            std::io::BufWriter::new(
                                std::fs::File::create(&level_path(&group.name, &name)).unwrap(),
                            ),
                            &game_state,
                            default(),
                        )
                        .unwrap();
                        group.levels.push(Level {
                            name,
                            preview: generate_preview(
                                &self.geng,
                                &self.assets,
                                &self.renderer,
                                &game_state,
                            ),
                        });
                        group.save_level_list();
                    }
                }
            }
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.clamp_camera();
        self.renderer.draw_background(framebuffer, &self.camera);
        for (group_index, group) in self.groups.iter().enumerate() {
            for (level_index, level) in group.levels.iter().enumerate() {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::TexturedQuad::new(
                        Aabb2::point(vec2(level_index, group_index).map(|x| x as f32 + 0.5))
                            .extend_symmetric(vec2::splat(self.config.level_icon_size / 2.0)),
                        &level.preview,
                    ),
                )
            }
            self.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "Plus",
                Rgba::WHITE,
                mat3::translate(vec2(group.levels.len(), group_index).map(|x| x as f32)),
            );
        }
        self.renderer.draw_tile(
            framebuffer,
            &self.camera,
            "Plus",
            Rgba::WHITE,
            mat3::translate(vec2(0, self.groups.len()).map(|x| x as f32)),
        );
        if let Some(selection) = self.hovered(self.geng.window().cursor_position()) {
            self.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "EditorSelect",
                Rgba::WHITE,
                mat3::translate(vec2(selection.level as f32, selection.group as f32)),
            );
            let text = match self.groups.get(selection.group) {
                Some(group) => match group.levels.get(selection.level) {
                    Some(_level) => format!("{}/{}", group.name, selection.level),
                    None => "New level".to_owned(),
                },
                None => "New group".to_owned(),
            };
            self.geng.default_font().draw_with_outline(
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

fn generate_preview(
    geng: &Geng,
    assets: &Assets,
    renderer: &Renderer,
    game_state: &GameState,
) -> ugli::Texture {
    let mut texture = ugli::Texture::new_uninitialized(
        geng.ugli(),
        vec2::splat(assets.config.editor.world.preview_texture_size),
    );
    texture.set_filter(ugli::Filter::Nearest);
    let bb = game_state.bounding_box().map(|x| x as f32);
    renderer.draw(
        &mut ugli::Framebuffer::new_color(
            geng.ugli(),
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
        &renderer.level_mesh(&game_state),
    );
    texture
}

impl State {
    // TODO: group these args into one Context struct
    pub fn load(
        geng: &Geng,
        assets: &Rc<Assets>,
        sound: &Rc<sound::State>,
        renderer: &Rc<Renderer>,
    ) -> impl geng::State {
        geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen::new(geng), {
            let geng = geng.clone();
            let assets = assets.clone();
            let sound = sound.clone();
            let renderer = renderer.clone();
            async move {
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
                    let levels =
                        future::join_all(level_names.into_iter().map(|level_name| async {
                            let game_state: GameState =
                                file::load_detect(level_path(&group_name, &level_name))
                                    .await
                                    .unwrap();
                            Level {
                                name: level_name,
                                preview: generate_preview(&geng, &assets, &renderer, &game_state),
                            }
                        }))
                        .await;
                    Group {
                        name: group_name,
                        levels,
                    }
                }))
                .await;
                let config = assets.config.editor.world.clone();
                Self {
                    geng: geng.clone(),
                    assets: assets.clone(),
                    sound: sound.clone(),
                    renderer: renderer.clone(),
                    framebuffer_size: vec2::splat(1.0),
                    groups: groups,
                    camera: geng::Camera2d {
                        center: vec2::ZERO,
                        rotation: 0.0,
                        fov: config.fov,
                    },
                    camera_controls: CameraControls::new(&geng, &assets.config.camera_controls),
                    config,
                    transition: None,
                }
            }
        })
    }
}
