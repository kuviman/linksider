use super::*;

#[derive(Deserialize)]
pub struct Controls {
    pub toggle: geng::Key,
    camera_drag: geng::MouseButton,
    create: geng::MouseButton,
    delete: geng::MouseButton,
    choose: geng::Key,
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
        }
    }

    fn screen_to_tile(&self, screen_pos: vec2<f64>) -> vec2<i32> {
        let world_pos = self
            .camera
            .screen_to_world(self.framebuffer_size, screen_pos.map(|x| x as f32));
        world_pos.map(|x| x.floor() as i32)
    }

    fn create(&mut self, pos: vec2<f64>) {}

    fn delete(&mut self, pos: vec2<f64>) {}
}

impl geng::State for State {
    fn update(&mut self, delta_time: f64) {
        let _delta_time = delta_time as f32;
    }
    fn transition(&mut self) -> Option<geng::state::Transition> {
        self.transition.take()
    }
    fn handle_event(&mut self, event: geng::Event) {
        let Some(controls) = &self.assets.config.controls.editor else {
            log::error!("How am I in the editor if no controls");
            return;
        };
        match event {
            geng::Event::KeyDown { key } => {
                if key == controls.toggle {
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
            "EditorSelect",
            Rgba::WHITE,
            mat3::translate(
                self.screen_to_tile(self.geng.window().cursor_position())
                    .map(|x| x as f32),
            ),
        );
    }
}
