use super::*;

pub struct State {
    ctx: Context,
    framebuffer_size: vec2<f32>,
    camera: Camera2d,
    transition: Option<Transition>,
    level_mesh: renderer::LevelMesh,
    history_player: history::Player,
    vfx: renderer::Vfx,
}

pub enum Transition {
    NextLevel,
    PrevLevel,
    Editor,
    Exit,
}

impl State {
    pub fn new(ctx: &Context, level: &Level) -> Self {
        let game_state = GameState::init(&ctx.assets.logic_config, level);
        Self {
            ctx: ctx.clone(),
            framebuffer_size: vec2::splat(1.0),
            camera: Camera2d {
                center: game_state.center(),
                rotation: Angle::ZERO,
                fov: ctx.assets.config.fov,
            },
            transition: None,
            level_mesh: ctx.renderer.level_mesh(level),
            history_player: history::Player::new(
                game_state,
                &ctx.assets.logic_config,
                ctx.assets.config.animation_time,
            ),
            vfx: renderer::Vfx::new(ctx),
        }
    }
    pub fn finish(&mut self, finish: Transition) {
        self.transition = Some(finish);
    }

    pub async fn run(mut self, actx: &mut async_states::Context) -> Transition {
        loop {
            match actx.wait().await {
                async_states::Event::Event(event) => self.handle_event(event),
                async_states::Event::Update(delta_time) => self.update(delta_time),
                async_states::Event::Draw => self.draw(&mut actx.framebuffer()),
            }
            if let Some(transition) = self.transition.take() {
                return transition;
            }
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;

        let is_pressed = |&key| self.ctx.geng.window().is_key_pressed(key);
        let input = if self.ctx.assets.config.controls.left.iter().any(is_pressed) {
            Some(Input::Left)
        } else if self.ctx.assets.config.controls.right.iter().any(is_pressed) {
            Some(Input::Right)
        } else if self.ctx.assets.config.controls.skip.iter().any(is_pressed) {
            Some(Input::Skip)
        } else {
            None
        };
        let timeline_input = if self.ctx.assets.config.controls.undo.iter().any(is_pressed) {
            Some(-1)
        } else if self.ctx.assets.config.controls.redo.iter().any(is_pressed) {
            Some(1)
        } else {
            None
        };
        let update = self.history_player.update(
            delta_time,
            &self.ctx.assets.logic_config,
            input,
            timeline_input,
        );
        if let Some(moves) = update.started {
            // TODO copypasta
            self.ctx.sound.play_turn_start_sounds(moves);
            self.vfx.add_moves(moves);
        }
        if let Some(moves) = update.finished {
            self.ctx.sound.play_turn_end_sounds(moves);
        }
        if let Some(entity) = self.history_player.frame().current_state.selected_entity() {
            self.camera.center = lerp(
                self.camera.center,
                entity.pos.cell.map(|x| x as f32 + 0.5),
                (delta_time * self.ctx.assets.config.camera_speed).min(1.0),
            );
        }
        if self.history_player.frame().current_state.finished() {
            self.finish(Transition::NextLevel);
        }

        self.vfx.update(delta_time);
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => {
                if key == self.ctx.assets.config.editor.level.controls.toggle {
                    self.finish(Transition::Editor);
                }

                if self.ctx.assets.config.controls.escape.contains(&key) {
                    self.finish(Transition::Exit);
                }

                if let Some(cheats) = &self.ctx.assets.config.controls.cheats {
                    if key == cheats.prev_level {
                        self.finish(Transition::PrevLevel);
                    } else if key == cheats.next_level {
                        self.finish(Transition::NextLevel);
                    }
                }

                if self.ctx.assets.config.controls.restart.contains(&key) {
                    self.history_player.restart();
                }
                if self.ctx.assets.config.controls.undo.contains(&key) {
                    self.history_player.undo();
                }
                if self.ctx.assets.config.controls.redo.contains(&key) {
                    self.history_player.redo();
                }

                let input = if self.ctx.assets.config.controls.left.contains(&key) {
                    Some(Input::Left)
                } else if self.ctx.assets.config.controls.right.contains(&key) {
                    Some(Input::Right)
                } else if self.ctx.assets.config.controls.skip.contains(&key) {
                    Some(Input::Skip)
                } else {
                    None
                };
                if let Some(input) = input {
                    if self.history_player.frame().animation.is_none() {
                        if let Some(moves) = self
                            .history_player
                            .process_move(&self.ctx.assets.logic_config, input)
                        {
                            self.ctx.sound.play_turn_start_sounds(moves);
                            self.vfx.add_moves(moves);
                        }
                    }
                }
                if self.ctx.assets.config.controls.next_player.contains(&key) {
                    self.history_player
                        .change_player_selection(&self.ctx.assets.logic_config, 1);
                    if let Some(player) =
                        self.history_player.frame().current_state.selected_entity()
                    {
                        self.vfx.change_player(player.pos);
                        self.ctx.sound.player_change();
                    }
                }
                if self.ctx.assets.config.controls.prev_player.contains(&key) {
                    self.history_player
                        .change_player_selection(&self.ctx.assets.logic_config, -1);
                    if let Some(player) =
                        self.history_player.frame().current_state.selected_entity()
                    {
                        self.vfx.change_player(player.pos);
                        self.ctx.sound.player_change();
                    }
                }
            }
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        let frame = self.history_player.frame();
        self.ctx
            .renderer
            .draw(framebuffer, &self.camera, frame, &self.level_mesh);
        self.vfx.draw(framebuffer, &self.camera);
    }
}
