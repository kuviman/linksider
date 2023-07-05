use super::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    player_change: Rc<SpriteSheet>,
    walk: Rc<SpriteSheet>,
    hit_wall: Rc<SpriteSheet>,
    jump: Rc<SpriteSheet>,
    happy: Rc<SpriteSheet>,
    slide: Rc<SpriteSheet>,
    zzz: Rc<SpriteSheet>,
}

struct Cell {
    sprite_sheet: Rc<SpriteSheet>,
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
                            sprite_sheet: assets.hit_wall.clone(),
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
                sprite_sheet: texture.clone(),
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
            sprite_sheet: self.ctx.assets.renderer.vfx.zzz.clone(),
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
            sprite_sheet: self.ctx.assets.renderer.vfx.player_change.clone(),
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
            self.ctx.renderer.draw_sprite_sheet(
                &cell.sprite_sheet,
                framebuffer,
                camera,
                Rgba::WHITE,
                cell.t,
                mat3::translate(cell.pos.cell.map(|x| x as f32 + 0.5))
                    * mat3::rotate(cell.pos.angle.to_angle())
                    * mat3::scale(vec2(if cell.flip { -1.0 } else { 1.0 }, 1.0)),
            );
        }
    }
}
