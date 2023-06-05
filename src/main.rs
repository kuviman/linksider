use geng::prelude::*;

use ldtk::Ldtk;

mod background;
mod config;
mod history;
mod sound;
mod util;

use config::Config;
use logicsider::*;
use util::*;

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: ugli::Program,
    pub fullscreen_texture: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub config: Config,
    #[load(serde, path = "logic.toml")]
    pub logic_config: logicsider::Config,
    pub world: Ldtk,
    pub shaders: Shaders,
    pub background: background::Assets,
    pub sound: sound::Assets,
    pub tileset: autotile::Tileset,
}

struct Game {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Rc<Assets>,
    level: usize,
    history_player: history::Player,
    camera: Camera2d,
    transition: Option<geng::state::Transition>,
    background: background::State,
    sound: Rc<sound::State>,
    level_mesh: ugli::VertexBuffer<draw2d::TexturedVertex>,
    entity_meshes: HashMap<String, ugli::VertexBuffer<draw2d::TexturedVertex>>,
}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, sound: &Rc<sound::State>, level: usize) -> Self {
        let game_state = GameState::from_ldtk(&assets.world.json, &assets.logic_config, level);
        let level_mesh = ugli::VertexBuffer::new_static(
            geng.ugli(),
            assets
                .tileset
                .def
                .generate_mesh({
                    struct TileMap<'a> {
                        state: &'a GameState,
                    }
                    impl autotile::TileMap for TileMap<'_> {
                        type NonEmptyIter<'a> = Box<dyn Iterator<Item = vec2<i32>> + 'a> where Self:'a ;
                        fn non_empty_tiles(&self) -> Self::NonEmptyIter<'_> {
                            Box::new(self.state.tiles.keys().copied())
                        }

                        fn get_at(&self, pos: vec2<i32>) -> Option<&str> {
                            Some(match self.state.tiles.get(&pos)? {
                                Tile::Nothing => return None,
                                Tile::Block => "block",
                                Tile::Disable => "disable",
                                Tile::Cloud => "cloud",
                            })
                        }
                    }
                    &TileMap { state: &game_state }
                })
                .flat_map(|tile| {
                    let uv = assets.tileset.def.uv(tile.tileset_pos, assets.tileset.texture.size());
                    let pos = Aabb2::point(tile.pos)
                        .extend_positive(vec2::splat(1))
                        .map(|x| x as f32);
                    let corners = pos.zip(uv).corners();
                    [corners[0], corners[1], corners[2], corners[0], corners[2], corners[3]]
                })
                .map(|vec2((pos_x, uv_x), (pos_y, uv_y))| draw2d::TexturedVertex {
                    a_pos: vec2(pos_x, pos_y),
                    a_color: Rgba::WHITE,
                    a_vt: vec2(uv_x, uv_y),
                })
                .collect(),
        );
        let history_player = history::Player::new(game_state, assets.config.animation_time);
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            level,
            framebuffer_size: vec2::splat(1.0),
            history_player,
            camera: Camera2d {
                center: vec2::ZERO,
                rotation: 0.0,
                fov: 200.0 / 16.0,
            },
            transition: None,
            background: background::State::new(geng, assets),
            sound: sound.clone(),
            level_mesh,
            entity_meshes: assets
                .tileset
                .def
                .tiles
                .iter()
                .filter_map(|(name, tile)| {
                    tile.default.map(|tileset_pos| {
                        (
                            name.to_owned(),
                            ugli::VertexBuffer::new_static(geng.ugli(), {
                                let uv = assets
                                    .tileset
                                    .def
                                    .uv(tileset_pos, assets.tileset.texture.size());
                                let pos = Aabb2::ZERO.extend_positive(vec2::splat(1.0));
                                let corners = pos.zip(uv).corners();
                                [
                                    corners[0], corners[1], corners[2], corners[0], corners[2],
                                    corners[3],
                                ]
                                .map(
                                    |vec2((pos_x, uv_x), (pos_y, uv_y))| draw2d::TexturedVertex {
                                        a_pos: vec2(pos_x, pos_y),
                                        a_color: Rgba::WHITE,
                                        a_vt: vec2(uv_x, uv_y),
                                    },
                                )
                                .to_vec()
                            }),
                        )
                    })
                })
                .collect(),
        }
    }
    pub fn draw_mesh(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        vertex_data: impl ugli::VertexDataSource,
        texture: &ugli::Texture,
        color: Rgba<f32>,
        matrix: mat3<f32>,
    ) {
        ugli::draw(
            framebuffer,
            &self.assets.shaders.texture,
            ugli::DrawMode::Triangles,
            vertex_data,
            (
                ugli::uniforms! {
                    u_model_matrix: matrix,
                    u_color: color,
                    u_texture: texture,
                },
                self.camera.uniforms(self.framebuffer_size),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()), // TODO premultiplied
                ..default()
            },
        );
    }

    pub fn change_level(&mut self, change: isize) {
        let new_level = self.level as isize + change;
        if (0..self.assets.world.levels.len() as isize).contains(&new_level) {
            self.transition = Some(geng::state::Transition::Switch(Box::new(Self::new(
                &self.geng,
                &self.assets,
                &self.sound,
                new_level as usize,
            ))));
        }
    }
}

impl geng::State for Game {
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

        self.background.draw(framebuffer, &self.camera);

        let history::Frame {
            current_state,
            animation,
        } = self.history_player.frame();
        let no_moves = Moves::default();
        let history::Animation {
            prev_state,
            moves,
            t,
        } = animation.unwrap_or(history::Animation {
            prev_state: current_state,
            moves: &no_moves,
            t: 0.0,
        });

        self.draw_mesh(
            framebuffer,
            &self.level_mesh,
            &self.assets.tileset.texture,
            Rgba::WHITE,
            mat3::identity(),
        );

        for goal in &prev_state.goals {
            self.draw_mesh(
                framebuffer,
                &self.entity_meshes["Goal"],
                &self.assets.tileset.texture,
                Rgba::WHITE,
                mat3::translate(goal.pos.cell.map(|x| x as f32 + 0.5))
                    * goal.pos.angle.to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }
        for entity in &prev_state.entities {
            let entity_move = moves.entity_moves.get(&entity.id);
            let (from, to) = match entity_move {
                Some(entity_move) => (entity_move.prev_pos, entity_move.new_pos),
                None => (entity.pos, entity.pos),
            };

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

            self.draw_mesh(
                framebuffer,
                &self.entity_meshes[&entity.identifier],
                &self.assets.tileset.texture,
                Rgba::WHITE,
                transform,
            );

            for (side_index, side) in entity.sides.iter().enumerate() {
                if let Some(effect) = &side.effect {
                    self.draw_mesh(
                        framebuffer,
                        &self.entity_meshes[&format!("{effect:?}Power")],
                        &self.assets.tileset.texture,
                        Rgba::WHITE,
                        transform
                            * mat3::rotate_around(
                                vec2::splat(0.5),
                                Entity::relative_side_angle(side_index).to_radians()
                                    - f32::PI / 2.0,
                            )
                            * mat3::translate(vec2(0.0, 1.0)),
                    );
                }
            }
        }
        for powerup in &prev_state.powerups {
            self.draw_mesh(
                framebuffer,
                &self.entity_meshes[&format!("{:?}Power", powerup.effect)],
                &self.assets.tileset.texture,
                Rgba::WHITE,
                mat3::translate(powerup.pos.cell.map(|x| x as f32 + 0.5))
                    * (powerup.pos.angle - IntAngle::DOWN).to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
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
