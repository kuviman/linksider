use super::*;

mod background;
mod vfx;

pub use vfx::Vfx;

#[derive(Deserialize)]
pub struct ShadowConfig {
    offset: vec2<f32>,
    opacity: f32,
}

#[derive(Deserialize)]
pub struct Config {
    shadow: ShadowConfig,
}

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
    vfx: vfx::Assets,
}

pub struct Renderer {
    geng: Geng,
    assets: Rc<crate::Assets>,
    background: background::State,
    entity_meshes: HashMap<String, ugli::VertexBuffer<draw2d::TexturedVertex>>,
    grid_mesh: ugli::VertexBuffer<draw2d::TexturedVertex>,
    white_texture: ugli::Texture,
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
            white_texture: ugli::Texture::new_with(geng.ugli(), vec2(1, 1), |_| Rgba::WHITE),
            grid_mesh: Self::create_grid_mesh(geng.ugli(), 100, 100),
        }
    }

    fn create_grid_mesh(
        ugli: &Ugli,
        width: usize,
        height: usize,
    ) -> ugli::VertexBuffer<draw2d::TexturedVertex> {
        let mut data = Vec::with_capacity(((width + 1) + (height + 1)) * 2);
        for x in 0..=width {
            data.push(draw2d::TexturedVertex {
                a_pos: vec2(x as f32, 0.0),
                a_color: Rgba::WHITE,
                a_vt: vec2::ZERO,
            });
            data.push(draw2d::TexturedVertex {
                a_pos: vec2(x as f32, height as f32),
                a_color: Rgba::WHITE,
                a_vt: vec2::ZERO,
            });
        }
        for y in 0..=height {
            data.push(draw2d::TexturedVertex {
                a_pos: vec2(0.0, y as f32),
                a_color: Rgba::WHITE,
                a_vt: vec2::ZERO,
            });
            data.push(draw2d::TexturedVertex {
                a_pos: vec2(width as f32, y as f32),
                a_color: Rgba::WHITE,
                a_vt: vec2::ZERO,
            });
        }
        ugli::VertexBuffer::new_static(ugli, data)
    }

    pub fn draw_grid(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        color: Rgba<f32>,
    ) {
        let bottom_left_world =
            camera.screen_to_world(framebuffer.size().map(|x| x as f32), vec2::ZERO);
        self.draw_mesh_impl(
            framebuffer,
            camera,
            &self.grid_mesh,
            ugli::DrawMode::Lines { line_width: 1.0 },
            &self.white_texture,
            color,
            mat3::translate(bottom_left_world.map(f32::floor)),
        );
    }

    pub fn draw_background(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
    ) {
        self.background.draw(framebuffer, camera);
    }

    pub fn draw_level(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        level: &Level,
        level_mesh: &LevelMesh,
    ) {
        // TODO not generate game state on every frame
        self.draw(
            framebuffer,
            camera,
            history::Frame {
                current_state: &GameState::init(&self.assets.logic_config, level),
                animation: None,
            },
            level_mesh,
        );
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

        self.draw_background(framebuffer, camera);

        // Shadow
        self.draw_colored(
            framebuffer,
            camera,
            current_state,
            prev_state,
            moves,
            t,
            level_mesh,
            mat3::translate(self.assets.config.render.shadow.offset),
            Rgba::new(0.0, 0.0, 0.0, self.assets.config.render.shadow.opacity),
        );

        for goal in &prev_state.goals {
            self.draw_tile(
                framebuffer,
                camera,
                "Goal",
                Rgba::WHITE,
                mat3::translate(goal.pos.cell.map(|x| x as f32 + 0.5))
                    * goal.pos.angle.to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }

        self.draw_colored(
            framebuffer,
            camera,
            current_state,
            prev_state,
            moves,
            t,
            level_mesh,
            mat3::identity(),
            Rgba::WHITE,
        );
    }

    fn draw_colored(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        current_state: &GameState,
        prev_state: &GameState,
        moves: &Moves,
        t: f32,
        level_mesh: &LevelMesh,
        transform: mat3<f32>,
        color: Rgba<f32>,
    ) {
        self.draw_mesh_impl(
            framebuffer,
            camera,
            &level_mesh.0,
            ugli::DrawMode::Triangles,
            &self.assets.renderer.tileset.texture,
            color,
            transform,
        );

        for entity in &prev_state.entities {
            let entity_move = moves.entity_moves.get(&entity.id);
            let mut animation_time = 1.0;
            let (from, to) = match entity_move {
                Some(entity_move) => {
                    if let EntityMoveType::Jump {
                        cells_travelled,
                        jump_force,
                        ..
                    } = entity_move.move_type
                    {
                        animation_time = cells_travelled as f32 / jump_force as f32;
                    }
                    (entity_move.prev_pos, entity_move.new_pos)
                }
                None => (entity.pos, entity.pos),
            };
            let t = (t / animation_time).min(1.0);

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
                        * mat3::rotate_around(vec2::splat(0.5), to.angle.to_angle());
                }
                let delta_pos = to_pos - from_pos;
                let delta_rot = to.angle.to_angle() - from.angle.to_angle();
                let rotation_origin = vec2::splat(0.5)
                    + from_pos
                    + delta_pos / 2.0
                    + delta_pos.rotate_90() / (delta_rot / 2.0).tan() / 2.0;

                let from_transform = mat3::translate(from_pos)
                    * mat3::rotate_around(vec2::splat(0.5), from.angle.to_angle());

                // Double border radius when doing 180 since there is also border radius on the
                // level geometry now
                let border_radius: f32 = delta_rot.abs().as_degrees() / 90.0 * border_radius;
                let extra_len = (1.0 / ((1.0 - (t - 0.5).abs() * 2.0) * f32::PI / 4.0).cos() - 1.0)
                    * border_radius;

                mat3::rotate_around(rotation_origin, delta_rot * t)
                    * mat3::translate(
                        (rotation_origin - (from_pos + vec2::splat(0.5))).normalize_or_zero()
                            * extra_len,
                    )
                    * from_transform
            }

            let entity_transform = cube_move_transform(
                from,
                to,
                self.assets.config.border_radius_pixels as f32
                    / self.assets.config.cell_pixel_size as f32,
                t,
            );

            // Static entities are cached in level mesh
            if !entity.properties.r#static {
                let mut color = color;
                if entity.properties.player && Some(entity.id) != current_state.selected_player {
                    color = Rgba::from_vec4(
                        color.to_vec4() * self.assets.config.deselected_player_color.to_vec4(),
                    );
                }
                self.draw_tile(
                    framebuffer,
                    camera,
                    &entity.identifier,
                    color,
                    transform * entity_transform,
                );
            }

            for (side_index, side) in entity.sides.iter().enumerate() {
                if let Some(effect) = &side.effect {
                    self.draw_tile(
                        framebuffer,
                        camera,
                        &format!("{effect:?}Power"),
                        color,
                        transform
                            * entity_transform
                            * mat3::rotate_around(
                                vec2::splat(0.5),
                                Entity::relative_side_angle(side_index).to_angle()
                                    - Angle::from_degrees(90.0),
                            )
                            * mat3::translate(vec2(0.0, 1.0)),
                    );
                }
            }
        }
        for powerup in &prev_state.powerups {
            self.draw_tile(
                framebuffer,
                camera,
                &format!("{:?}Power", powerup.effect),
                color,
                transform
                    * mat3::translate(powerup.pos.cell.map(|x| x as f32 + 0.5))
                    * (powerup.pos.angle - IntAngle::DOWN).to_matrix()
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }
    }

    pub fn draw_tile(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        name: &str,
        color: Rgba<f32>,
        matrix: mat3<f32>,
    ) {
        let Some(vertex_data) = self.entity_meshes.get(name) else {
            log::error!("No data for rendering {name:?}");
            return;
        };
        self.draw_mesh_impl(
            framebuffer,
            camera,
            vertex_data,
            ugli::DrawMode::Triangles,
            &self.assets.renderer.tileset.texture,
            color,
            matrix,
        )
    }

    fn draw_mesh_impl(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        vertex_data: impl ugli::VertexDataSource,
        mode: ugli::DrawMode,
        texture: &ugli::Texture,
        color: Rgba<f32>,
        matrix: mat3<f32>,
    ) {
        ugli::draw(
            framebuffer,
            &self.assets.renderer.shaders.texture,
            mode,
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
    pub fn level_mesh(&self, level: &Level) -> LevelMesh {
        struct TileMap<'a> {
            config: &'a logicsider::Config,
            level: &'a Level,
        }
        impl autotile::TileMap for TileMap<'_> {
            type NonEmptyIter<'a> = Box<dyn Iterator<Item = vec2<i32>> + 'a> where Self:'a ;
            fn non_empty_tiles(&self) -> Self::NonEmptyIter<'_> {
                Box::new(
                    self.level
                        .entities
                        .iter()
                        .filter(|entity| self.config.entities[&entity.identifier].r#static)
                        .map(|entity| entity.pos.cell),
                )
            }

            fn get_at(&self, pos: vec2<i32>) -> Option<&str> {
                self.level
                    .entities
                    .iter()
                    .find(|entity| entity.pos.cell == pos)
                    .map(|entity| entity.identifier.as_str())
            }
        }
        LevelMesh(ugli::VertexBuffer::new_static(
            self.geng.ugli(),
            self.assets
                .renderer
                .tileset
                .def
                .generate_mesh(&TileMap {
                    config: &self.assets.logic_config,
                    level,
                })
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
