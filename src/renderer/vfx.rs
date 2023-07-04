use super::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    player_change: Rc<Texture>,
    walk: Rc<Texture>,
    hit_wall: Rc<Texture>,
    jump: Rc<Texture>,
    happy: Rc<Texture>,
    slide: Rc<Texture>,
    zzz: Rc<Texture>,
}

struct Cell {
    texture: Rc<Texture>,
    pos: Position,
    flip: bool,
    t: f32,
}

pub struct Vfx {
    ctx: Context,
    cells: Vec<Cell>,
}

impl Vfx {
    pub fn new(ctx: &Context) -> Self {
        Self {
            ctx: ctx.clone(),
            cells: default(),
        }
    }
    pub fn add_moves(&mut self, moves: &Moves) {
        let assets = &self.ctx.assets.renderer.vfx;
        for entity_move in &moves.entity_moves {
            let (angle, texture) = match entity_move.move_type {
                EntityMoveType::Magnet { magnet_angle, .. } => (magnet_angle, &assets.walk),
                EntityMoveType::EnterGoal { .. } => continue,
                EntityMoveType::Gravity => continue,
                EntityMoveType::Move => {
                    if entity_move.prev_pos.cell == entity_move.new_pos.cell {
                        continue;
                    }
                    (IntAngle::DOWN, &assets.walk)
                }
                EntityMoveType::Pushed => continue,
                EntityMoveType::SlideStart => (IntAngle::DOWN, &assets.slide),
                EntityMoveType::SlideContinue => (IntAngle::DOWN, &assets.slide),
                EntityMoveType::Jump {
                    from,
                    blocked_angle,
                    cells_traveled: cells_travelled,
                    jump_force,
                } => {
                    if let Some(blocked_angle) = blocked_angle {
                        self.cells.push(Cell {
                            texture: assets.hit_wall.clone(),
                            pos: Position {
                                cell: entity_move.new_pos.cell,
                                angle: blocked_angle.rotate_clockwise(),
                            },
                            flip: false,
                            t: -(cells_travelled as f32 / jump_force as f32),
                        });
                    }
                    (
                        from,
                        if self.ctx.assets.config.happy {
                            &assets.happy
                        } else {
                            &assets.jump
                        },
                    )
                }
                EntityMoveType::MagnetContinue => continue,
            };
            self.cells.push(Cell {
                texture: texture.clone(),
                pos: Position {
                    cell: entity_move.prev_pos.cell,
                    angle: angle.rotate_counter_clockwise(),
                },
                flip: entity_move.used_input == Input::Left,
                t: 0.0,
            });
        }
    }

    pub fn zzz(&mut self, cell: vec2<i32>) {
        self.cells.push(Cell {
            texture: self.ctx.assets.renderer.vfx.zzz.clone(),
            pos: Position {
                cell,
                angle: IntAngle::ZERO,
            },
            flip: false,
            t: 0.0,
        });
    }

    pub fn change_player(&mut self, pos: Position) {
        self.cells.push(Cell {
            texture: self.ctx.assets.renderer.vfx.player_change.clone(),
            pos: Position {
                cell: pos.cell,
                angle: IntAngle::ZERO,
            },
            flip: false,
            t: 0.0,
        });
    }

    pub fn update(&mut self, delta_time: f32) {
        for cell in &mut self.cells {
            cell.t += delta_time / self.ctx.assets.config.animation_time;
        }
        self.cells.retain(|cell| cell.t < 1.0);
    }

    pub fn draw(&self, framebuffer: &mut ugli::Framebuffer, camera: &impl geng::AbstractCamera2d) {
        for cell in &self.cells {
            if cell.t < 0.0 {
                continue;
            }
            let texture: &ugli::Texture = &cell.texture;
            assert!(texture.size().y % texture.size().x == 0);
            let frames = texture.size().y / texture.size().x;
            let frame = (cell.t * frames as f32).floor();
            let start_vt = frame / frames as f32;
            let end_vt = (frame + 1.0) / frames as f32;
            let (start_vt, end_vt) = (1.0 - end_vt, 1.0 - start_vt);
            let uv_aabb = Aabb2 {
                min: vec2(0.0, start_vt),
                max: vec2(1.0, end_vt),
            };
            let v = |x, y| TilesetVertex {
                a_pos: vec2(x as f32, y as f32),
                a_uv: uv_aabb.bottom_left()
                    + uv_aabb.size()
                        * vec2(if cell.flip { 1.0 - x as f32 } else { x as f32 }, y as f32),
            };
            let vertex_data = ugli::VertexBuffer::new_dynamic(
                self.ctx.geng.ugli(),
                vec![v(0, 0), v(1, 0), v(1, 1), v(0, 1)],
            );
            self.ctx.renderer.draw_mesh_impl(
                framebuffer,
                camera,
                &vertex_data,
                ugli::DrawMode::TriangleFan,
                texture,
                Rgba::WHITE,
                mat3::translate(cell.pos.cell.map(|x| x as f32 + 0.5))
                    * mat3::scale_uniform(
                        texture.size().x as f32 / self.ctx.assets.config.cell_pixel_size as f32,
                    )
                    * mat3::rotate(cell.pos.angle.to_angle())
                    * mat3::translate(vec2::splat(-0.5)),
            );
        }
    }
}
