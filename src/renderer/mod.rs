use super::*;

mod background;

#[derive(geng::asset::Load)]
struct Shaders {
    texture: ugli::Program,
    fullscreen_texture: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    shaders: Shaders,
    background: background::Assets,
    tileset: autotile::Tileset,
}

pub struct Renderer {
    geng: Geng,
    assets: Rc<crate::Assets>,
    background: background::State,
    entity_meshes: HashMap<String, ugli::VertexBuffer<draw2d::TexturedVertex>>,
}

impl Renderer {
    pub fn new(geng: &Geng, assets: &Rc<crate::Assets>) -> Self {
        let tileset = &assets.renderer.tileset;
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            background: background::State::new(geng, assets),
            entity_meshes: tileset
                .def
                .tiles
                .iter()
                .filter_map(|(name, tile)| {
                    tile.default.map(|tileset_pos| {
                        (
                            name.to_owned(),
                            ugli::VertexBuffer::new_static(geng.ugli(), {
                                let uv = tileset.def.uv(tileset_pos, tileset.texture.size());
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

    pub fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        frame: history::Frame,
        level_mesh: &LevelMesh,
    ) {
        let history::Frame {
            current_state,
            animation,
        } = frame;

        self.background.draw(framebuffer, camera);

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
            camera,
            &level_mesh.0,
            &self.assets.renderer.tileset.texture,
            Rgba::WHITE,
            mat3::identity(),
        );

        for goal in &prev_state.goals {
            self.draw_mesh(
                framebuffer,
                camera,
                &self.entity_meshes["Goal"],
                &self.assets.renderer.tileset.texture,
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
                camera,
                &self.entity_meshes[&entity.identifier],
                &self.assets.renderer.tileset.texture,
                Rgba::WHITE,
                transform,
            );

            for (side_index, side) in entity.sides.iter().enumerate() {
                if let Some(effect) = &side.effect {
                    self.draw_mesh(
                        framebuffer,
                        camera,
                        &self.entity_meshes[&format!("{effect:?}Power")],
                        &self.assets.renderer.tileset.texture,
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
                camera,
                &self.entity_meshes[&format!("{:?}Power", powerup.effect)],
                &self.assets.renderer.tileset.texture,
                Rgba::WHITE,
                mat3::translate(powerup.pos.cell.map(|x| x as f32 + 0.5))
                    * (powerup.pos.angle - IntAngle::DOWN).to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }
    }

    pub fn draw_mesh(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        vertex_data: impl ugli::VertexDataSource,
        texture: &ugli::Texture,
        color: Rgba<f32>,
        matrix: mat3<f32>,
    ) {
        ugli::draw(
            framebuffer,
            &self.assets.renderer.shaders.texture,
            ugli::DrawMode::Triangles,
            vertex_data,
            (
                ugli::uniforms! {
                    u_model_matrix: matrix,
                    u_color: color,
                    u_texture: texture,
                },
                camera.uniforms(framebuffer.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()), // TODO premultiplied
                ..default()
            },
        );
    }
}

pub struct LevelMesh(ugli::VertexBuffer<draw2d::TexturedVertex>);

impl Renderer {
    pub fn level_mesh(&self, state: &GameState) -> LevelMesh {
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
        LevelMesh(ugli::VertexBuffer::new_static(
            self.geng.ugli(),
            self.assets
                .renderer
                .tileset
                .def
                .generate_mesh(&TileMap { state })
                .flat_map(|tile| {
                    let uv = self.assets.renderer.tileset.def.uv(
                        tile.tileset_pos,
                        self.assets.renderer.tileset.texture.size(),
                    );
                    let pos = Aabb2::point(tile.pos)
                        .extend_positive(vec2::splat(1))
                        .map(|x| x as f32);
                    let corners = pos.zip(uv).corners();
                    [
                        corners[0], corners[1], corners[2], corners[0], corners[2], corners[3],
                    ]
                })
                .map(
                    |vec2((pos_x, uv_x), (pos_y, uv_y))| draw2d::TexturedVertex {
                        a_pos: vec2(pos_x, pos_y),
                        a_color: Rgba::WHITE,
                        a_vt: vec2(uv_x, uv_y),
                    },
                )
                .collect(),
        ))
    }
}
