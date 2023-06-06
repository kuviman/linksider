use super::*;

#[derive(Deserialize)]
pub struct Config {
    fov: f32,
    level_icon_size: f32,
    margin: f32,
}

struct Level {
    path: std::path::PathBuf,
    // TODO preview
}

fn level_path(group_name: &str, level_index: usize) -> std::path::PathBuf {
    group_dir(group_name).join(format!("{level_index}.ron"))
}

struct Group {
    name: String,
    levels: Vec<Level>,
}

fn group_dir(group_name: &str) -> std::path::PathBuf {
    run_dir().join("assets").join(group_name)
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

    fn hovered_level(&self, screen_pos: vec2<f64>) -> Option<Selection> {
        let world_pos = self.camera.screen_to_world(
            self.geng.window().size().map(|x| x as f32),
            screen_pos.map(|x| x as f32),
        );
        for (group_index, group) in self.groups.iter().enumerate() {
            for level_index in 0..group.levels.len() {
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
            geng::Event::MouseDown { position, button } => {
                if let Some(selection) = self.hovered_level(position) {
                    self.transition = Some(geng::state::Transition::Switch(Box::new(
                        editor::level::State::load(
                            &self.geng,
                            &self.assets,
                            &self.sound,
                            &self.renderer,
                            level_path(&self.groups[selection.group].name, selection.level),
                        ),
                    )))
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
            for (level_index, _level) in group.levels.iter().enumerate() {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::Quad::new(
                        Aabb2::point(vec2(level_index, group_index).map(|x| x as f32 + 0.5))
                            .extend_symmetric(vec2::splat(self.config.level_icon_size / 2.0)),
                        Rgba::GRAY,
                    ),
                )
            }
        }
        if let Some(selection) = self.hovered_level(self.geng.window().cursor_position()) {
            self.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "EditorSelect",
                Rgba::WHITE,
                mat3::translate(vec2(selection.level as f32, selection.group as f32)),
            );
            self.geng.default_font().draw_with_outline(
                framebuffer,
                &self.camera,
                &format!("{}/{}", self.groups[selection.group].name, selection.level),
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(vec2(selection.level as f32 + 0.5, selection.group as f32 + 1.5)),
                Rgba::WHITE,
                0.05,
                Rgba::BLACK,
            );
        }
    }
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
                let group_names: Vec<String> =
                    file::load_detect(run_dir().join("levels").join("groups.ron"))
                        .await
                        .unwrap();
                let groups = future::join_all(group_names.into_iter().map(|group_name| async {
                    let level_count: usize =
                        file::load_string(group_dir(&group_name).join("count.txt"))
                            .await
                            .unwrap()
                            .parse()
                            .unwrap();
                    let levels = (0..level_count)
                        .map(|index| Level {
                            path: level_path(&group_name, index),
                        })
                        .collect();
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
