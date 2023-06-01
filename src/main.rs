use geng::prelude::*;
use ldtk::Ldtk;

mod logic;
mod util;

use logic::*;
use util::*;

#[derive(Deserialize)]
pub struct CheatsControlsConfig {
    pub prev_level: Vec<geng::Key>,
    pub next_level: Vec<geng::Key>,
}

#[derive(Deserialize)]
pub struct ControlsConfig {
    pub left: Vec<geng::Key>,
    pub right: Vec<geng::Key>,
    pub skip: Vec<geng::Key>,
    pub next_player: Vec<geng::Key>,
    pub prev_player: Vec<geng::Key>,
    pub cheats: Option<CheatsControlsConfig>,
}

#[derive(Deserialize)]
pub struct Config {
    pub camera_speed: f32,
    pub animation_time: f32,
    pub controls: ControlsConfig,
}

// TODO #[load(serde)]
impl geng::asset::Load for Config {
    fn load(_manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        file::load_detect(path.to_owned()).boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("toml");
}

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub config: Config,
    pub world: Ldtk,
    pub shaders: Shaders,
}

struct Animation {
    r#move: Move,
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
}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, level: usize) -> Self {
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
                new_index as usize,
            ))));
        }
    }

    fn maybe_start_animation(&mut self, input: Input) {
        let r#move = self.state.check_move(input);
        log::debug!("{move:?}");
        if let Some(r#move) = r#move {
            self.animation = Some(Animation { r#move, t: 0.0 });
        }
    }
}

impl geng::State for Game {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;

        if let Some(animation) = &mut self.animation {
            animation.t += delta_time / self.assets.config.animation_time;
            if animation.t >= 1.0 {
                self.state.perform_move(&animation.r#move);
                self.animation = None;

                let is_pressed = |&key| self.geng.window().is_key_pressed(key);
                let input = if self.assets.config.controls.left.iter().any(is_pressed) {
                    Input::Left
                } else if self.assets.config.controls.right.iter().any(is_pressed) {
                    Input::Right
                } else {
                    Input::Skip
                };
                self.maybe_start_animation(input); // TODO: keep or revert???
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
                if let Some(input) = match key {
                    geng::Key::Left => Some(Input::Left),
                    geng::Key::Right => Some(Input::Right),
                    geng::Key::Space => Some(Input::Skip),
                    _ => None,
                } {
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

                if let Some(cheats) = &self.assets.config.controls.cheats {
                    if cheats.prev_level.contains(&key) {
                        self.change_level(-1);
                    } else if cheats.next_level.contains(&key) {
                        self.change_level(1);
                    }
                }
            }
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        for layer in &self.state.level.layers {
            if let Some(mesh) = &layer.mesh {
                self.draw_mesh(framebuffer, mesh, Rgba::WHITE, mat3::identity());
            }
        }
        for (index, player) in self.state.players.iter().enumerate() {
            let from = player.pos;
            let to = self
                .animation
                .as_ref()
                .and_then(|animation| animation.r#move.players.get(&index))
                .copied()
                .unwrap_or(from);
            let t = self.animation.as_ref().map_or(0.0, |animation| animation.t);

            self.draw_mesh(
                framebuffer,
                &player.mesh,
                Rgba::WHITE,
                mat3::translate(lerp(
                    from.cell.map(|x| x as f32 + 0.5),
                    to.cell.map(|x| x as f32 + 0.5),
                    t,
                )) * lerp(from.rot.to_matrix(), to.rot.to_matrix(), t)
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
        let assets: Assets = geng
            .asset_manager()
            .load(run_dir().join("assets"))
            .await
            .unwrap();
        Game::new(&geng, &Rc::new(assets), 0)
    });
}
