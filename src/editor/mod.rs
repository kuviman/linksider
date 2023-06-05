use super::*;

#[derive(Deserialize)]
pub struct Controls {
    pub toggle: geng::Key,
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
                center: vec2::ZERO,
                rotation: 0.0,
                fov: 200.0 / 16.0,
            },
            transition: None,
            sound: sound.clone(),
            renderer: renderer.clone(),
            level_mesh: renderer.level_mesh(&game_state),
            game_state,
            finish_callback,
        }
    }
}

impl geng::State for State {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
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
    }
}
