use geng::prelude::*;
use ldtk::Ldtk;

mod background;
mod config;
mod int_angle;
mod logic;
mod sound;
mod util;

use config::Config;
use int_angle::*;
use logic::*;
use util::*;

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: ugli::Program,
    pub fullscreen_texture: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub config: Config,
    pub world: Ldtk,
    pub shaders: Shaders,
    pub background: background::Assets,
    pub sound: sound::Assets,
}

struct Animation {
    moves: Moves,
    t: f32,
}

struct Game {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Rc<Assets>,
    state: GameState,
    camera: Camera2d,
    animation: Option<Animation>,
    transition: Option<geng::state::Transition>,
    background: background::State,
    sound: Rc<sound::State>,
}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, sound: &Rc<sound::State>, level: usize) -> Self {
        let level = &assets.world.levels[level];
        let mut result = Self {
            geng: geng.clone(),
            assets: assets.clone(),
            framebuffer_size: vec2::splat(1.0),
            state: GameState::new(level),
            camera: Camera2d {
                center: vec2::ZERO,
                rotation: 0.0,
                fov: 200.0 / 16.0,
            },
            animation: None,
            transition: None,
            background: background::State::new(geng, assets),
            sound: sound.clone(),
        };
        result.maybe_start_animation(Input::Skip);
        result
    }
    pub fn draw_mesh(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        mesh: &ldtk::Mesh,
        color: Rgba<f32>,
        matrix: mat3<f32>,
    ) {
        ugli::draw(
            framebuffer,
            &self.assets.shaders.texture,
            ugli::DrawMode::Triangles,
            &mesh.vertex_data,
            (
                ugli::uniforms! {
                    u_model_matrix: matrix,
                    u_color: color,
                    u_texture: &*mesh.texture,
                },
                self.camera.uniforms(self.framebuffer_size),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()), // TODO premultiplied
                ..default()
            },
        );
    }

    pub fn restart(&mut self) {
        self.change_level(0);
    }

    pub fn change_level(&mut self, change: isize) {
        let current_index = self
            .assets
            .world
            .levels
            .iter()
            .position(|level| Rc::ptr_eq(level, &self.state.level))
            .unwrap();
        let new_index = current_index as isize + change;
        if (0..self.assets.world.levels.len() as isize).contains(&new_index) {
            self.transition = Some(geng::state::Transition::Switch(Box::new(Self::new(
                &self.geng,
                &self.assets,
                &self.sound,
                new_index as usize,
            ))));
        }
    }

    fn maybe_start_animation(&mut self, input: Input) {
        let moves = self.state.process_turn(input);
        log::debug!("{moves:?}");
        if let Some(moves) = moves {
            self.animation = Some(Animation { moves, t: 0.0 });
        }
    }
}

impl geng::State for Game {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;

        if let Some(animation) = &mut self.animation {
            animation.t += delta_time / self.assets.config.animation_time;
            if animation.t >= 1.0 {
                self.state.perform_moves(&animation.moves);
                self.animation = None;

                if self.state.finished() {
                    self.change_level(1);
                } else {
                    let is_pressed = |&key| self.geng.window().is_key_pressed(key);
                    let input = if self.assets.config.controls.left.iter().any(is_pressed) {
                        Input::Left
                    } else if self.assets.config.controls.right.iter().any(is_pressed) {
                        Input::Right
                    } else {
                        Input::Skip
                    };
                    self.maybe_start_animation(input); // TODO: keep or revert input???
                }
            }
        }

        self.camera.center = lerp(
            self.camera.center,
            self.state
                .selected_player()
                .pos
                .cell
                .map(|x| x as f32 + 0.5),
            (delta_time * self.assets.config.camera_speed).min(1.0),
        );
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
                    } else {
                        let direction = match key {
                            geng::Key::Left => Some(IntAngle::LEFT),
                            geng::Key::Right => Some(IntAngle::RIGHT),
                            geng::Key::Down => Some(IntAngle::DOWN),
                            geng::Key::Up => Some(IntAngle::UP),
                            _ => None,
                        };
                        if let Some(direction) = direction {
                            let effect = if self.geng.window().is_key_pressed(cheats.effect.jump) {
                                Some(Some(Effect::Jump))
                            } else if self.geng.window().is_key_pressed(cheats.effect.delete) {
                                Some(None)
                            } else {
                                None
                            };
                            if let Some(effect) = effect {
                                let player = self.state.selected_player_mut();
                                player.sides[player.side_index(direction)].effect = effect;
                                self.maybe_start_animation(Input::Skip);
                            }
                        }
                    }
                }

                if self.assets.config.controls.restart.contains(&key) {
                    self.restart();
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
                    if self.animation.is_none() {
                        self.maybe_start_animation(input);
                    }
                }
                if self.assets.config.controls.next_player.contains(&key) {
                    self.state.change_player_selection(1);
                }
                if self.assets.config.controls.prev_player.contains(&key) {
                    self.state.change_player_selection(-1);
                }
            }
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);

        self.background.draw(framebuffer, &self.camera);

        for layer in &self.state.level.layers {
            if let Some(mesh) = &layer.mesh {
                self.draw_mesh(framebuffer, mesh, Rgba::WHITE, mat3::identity());
            }
        }
        for goal in &self.state.goals {
            self.draw_mesh(
                framebuffer,
                &goal.mesh,
                Rgba::WHITE,
                mat3::translate(goal.pos.cell.map(|x| x as f32 + 0.5))
                    * goal.pos.angle.to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }
        for (index, player) in self.state.players.iter().enumerate() {
            let from = player.pos;
            let to = self
                .animation
                .as_ref()
                .and_then(|animation| animation.moves.players.get(&index))
                .map(|player_move| player_move.new_pos)
                .unwrap_or(from);
            let t = self.animation.as_ref().map_or(0.0, |animation| animation.t);

            fn cube_move_transform(
                from: Position,
                to: Position,
                border_radius: f32,
                t: f32,
            ) -> mat3<f32> {
                let from_pos = from.cell.map(|x| x as f32);
                let to_pos = to.cell.map(|x| x as f32);
                if from.angle == to.angle {
                    return mat3::translate(lerp(from_pos, to_pos, t))
                        * mat3::rotate_around(vec2::splat(0.5), to.angle.to_radians());
                }
                let delta_pos = to_pos - from_pos;
                let delta_rot = to.angle.to_radians() - from.angle.to_radians();
                let rotation_origin = vec2::splat(0.5)
                    + from_pos
                    + delta_pos / 2.0
                    + delta_pos.rotate_90() / (delta_rot / 2.0).tan() / 2.0;

                let from_transform = mat3::translate(from_pos)
                    * mat3::rotate_around(vec2::splat(0.5), from.angle.to_radians());

                // Double border radius when doing 180 since there is also border radius on the
                // level geometry now
                let border_radius: f32 = delta_rot.abs() / (f32::PI / 2.0) * border_radius;
                let extra_len = (1.0 / ((1.0 - (t - 0.5).abs() * 2.0) * f32::PI / 4.0).cos() - 1.0)
                    * border_radius;

                mat3::rotate_around(rotation_origin, delta_rot * t)
                    * mat3::translate(
                        (rotation_origin - (from_pos + vec2::splat(0.5))).normalize_or_zero()
                            * extra_len,
                    )
                    * from_transform

                //
                // *transform = Transform::from_translation(prev_pos.extend(transform.translation.z))
                //     .with_rotation(Quat::from_rotation_z(prev_rot));
                // transform.rotate_around(
                //     rotation_origin.extend(123.45),
                //     Quat::from_rotation_z(delta_rot * t),
                // );
                // transform.translation = (transform.translation.xy()
                //     + (rotation_origin - transform.translation.xy()).normalize_or_zero()
                //         * extra_len)
                //     .extend(transform.translation.z);
            }

            let transform = cube_move_transform(
                from,
                to,
                self.assets.config.border_radius_pixels as f32
                    / self.assets.config.cell_pixel_size as f32,
                t,
            );

            self.draw_mesh(framebuffer, &player.mesh, Rgba::WHITE, transform);

            for (side_index, side) in player.sides.iter().enumerate() {
                let transform = transform
                    // TODO: mat3::rotate_around
                    * mat3::translate(vec2::splat(0.5))
                    * mat3::rotate(Player::relative_side_angle(side_index).to_radians() - f32::PI / 2.0)
                    * mat3::translate(vec2(-0.5, 0.5));
                if let Some(effect) = &side.effect {
                    // TODO: mesh should be found differently
                    let mesh = self
                        .state
                        .level
                        .layers
                        .iter()
                        .flat_map(|layer| &layer.entities)
                        .find_map(|entity| {
                            if entity.identifier == format!("{effect:?}Power") {
                                Some(&entity.mesh)
                            } else {
                                None
                            }
                        })
                        .expect("Failed to find mesh");
                    self.draw_mesh(framebuffer, mesh, Rgba::WHITE, transform);
                }
            }
        }
        for powerup in &self.state.powerups {
            self.draw_mesh(
                framebuffer,
                &powerup.mesh,
                Rgba::WHITE,
                mat3::translate(powerup.pos.cell.map(|x| x as f32 + 0.5))
                    * (powerup.pos.angle - IntAngle::DOWN).to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }

        if false {
            for layer in &self.state.level.layers {
                if let Some(grid) = &layer.int_grid {
                    for (&pos, value) in grid {
                        self.geng.draw2d().draw2d(
                            framebuffer,
                            &self.camera,
                            &draw2d::Text::unit(
                                &**self.geng.default_font(),
                                format!("{value:?}"),
                                Rgba::WHITE,
                            )
                            .scale_uniform(0.1)
                            .translate(pos.map(|x| x as f32 + 0.5)),
                        );
                    }
                }
            }
        }
    }
}

fn main() {
    logger::init();
    geng::setup_panic_handler();
    let geng = Geng::new("linksider");
    geng.clone().run_loading(async move {
        let geng = &geng;
        let assets: Assets = geng
            .asset_manager()
            .load(run_dir().join("assets"))
            .await
            .unwrap();
        let assets = Rc::new(assets);
        let assets = &assets;
        let sound = Rc::new(sound::State::new(&geng, assets));
        Game::new(geng, assets, &sound, 0)
    });
}
