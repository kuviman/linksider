use super::*;

pub struct State {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Rc<Assets>,
    level: usize,
    history_player: history::Player,
    camera: Camera2d,
    transition: Option<geng::state::Transition>,
    sound: Rc<sound::State>,
    renderer: Rc<Renderer>,
    level_mesh: renderer::LevelMesh,
}

impl State {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        renderer: &Rc<Renderer>,
        sound: &Rc<sound::State>,
        level: usize,
    ) -> Self {
        let game_state = GameState::from_ldtk(&assets.world.json, &assets.logic_config, level);
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            level,
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
            history_player: history::Player::new(game_state, assets.config.animation_time),
        }
    }
    pub fn change_level(&mut self, change: isize) {
        let new_level = self.level as isize + change;
        if (0..self.assets.world.levels.len() as isize).contains(&new_level) {
            self.transition = Some(geng::state::Transition::Switch(Box::new(Self::new(
                &self.geng,
                &self.assets,
                &self.renderer,
                &self.sound,
                new_level as usize,
            ))));
        }
    }
}

impl geng::State for State {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;

        let is_pressed = |&key| self.geng.window().is_key_pressed(key);
        let input = if self.assets.config.controls.left.iter().any(is_pressed) {
            Some(Input::Left)
        } else if self.assets.config.controls.right.iter().any(is_pressed) {
            Some(Input::Right)
        } else if self.assets.config.controls.skip.iter().any(is_pressed) {
            Some(Input::Skip)
        } else {
            None
        };
        let timeline_input = if self.assets.config.controls.undo.iter().any(is_pressed) {
            Some(-1)
        } else if self.assets.config.controls.redo.iter().any(is_pressed) {
            Some(1)
        } else {
            None
        };
        let update = self
            .history_player
            .update(delta_time, input, timeline_input);
        if let Some(moves) = update.started {
            self.sound.play_turn_start_sounds(moves);
        }
        if let Some(moves) = update.finished {
            self.sound.play_turn_end_sounds(moves);
        }
        if let Some(entity) = self.history_player.frame().current_state.selected_entity() {
            self.camera.center = lerp(
                self.camera.center,
                entity.pos.cell.map(|x| x as f32 + 0.5),
                (delta_time * self.assets.config.camera_speed).min(1.0),
            );
        }
        if self.history_player.frame().current_state.finished() {
            self.change_level(1);
        }
    }
    fn transition(&mut self) -> Option<geng::state::Transition> {
        self.transition.take()
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => {
                if let Some(cheats) = &self.assets.config.controls.cheats {
                    if key == cheats.prev_level {
                        self.change_level(-1);
                    } else if key == cheats.next_level {
                        self.change_level(1);
                    }
                }

                if self.assets.config.controls.restart.contains(&key) {
                    self.history_player.restart();
                }
                if self.assets.config.controls.undo.contains(&key) {
                    self.history_player.undo();
                }
                if self.assets.config.controls.redo.contains(&key) {
                    self.history_player.redo();
                }

                let input = if self.assets.config.controls.left.contains(&key) {
                    Some(Input::Left)
                } else if self.assets.config.controls.right.contains(&key) {
                    Some(Input::Right)
                } else if self.assets.config.controls.skip.contains(&key) {
                    Some(Input::Skip)
                } else {
                    None
                };
                if let Some(input) = input {
                    if self.history_player.frame().animation.is_none() {
                        if let Some(moves) = self.history_player.process_move(input) {
                            self.sound.play_turn_start_sounds(moves);
                        }
                    }
                }
                if self.assets.config.controls.next_player.contains(&key) {
                    self.history_player.change_player_selection(1);
                }
                if self.assets.config.controls.prev_player.contains(&key) {
                    self.history_player.change_player_selection(-1);
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
            self.history_player.frame(),
            &self.level_mesh,
        );
    }
}
