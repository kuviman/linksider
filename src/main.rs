use geng::prelude::*;
use ldtk::Ldtk;

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub world: Ldtk,
    pub shaders: Shaders,
}

struct Game {
    framebuffer_size: vec2<f32>,
    geng: Geng,
    assets: Rc<Assets>,
    level: usize,
    camera: Camera2d,
}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            framebuffer_size: vec2::splat(1.0),
            level: 0,
            camera: Camera2d {
                center: vec2::ZERO,
                rotation: 0.0,
                fov: 200.0 / 16.0,
            },
        }
    }
}

impl geng::State for Game {
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Left => {
                    if self.level > 0 {
                        self.level -= 1;
                    }
                }
                geng::Key::Right => {
                    if self.level + 1 < self.assets.world.levels.len() {
                        self.level += 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        let level = &self.assets.world.levels[self.level];

        if let Some(player) = level
            .layers
            .iter()
            .flat_map(|layer| &layer.entities)
            .find(|entity| entity.identifier == "Player")
        {
            self.camera.center = player.pos.map(|x| x as f32 + 0.5);
        }

        for layer in &level.layers {
            if let Some(mesh) = &layer.mesh {
                ugli::draw(
                    framebuffer,
                    &self.assets.shaders.texture,
                    ugli::DrawMode::Triangles,
                    &mesh.vertex_data,
                    (
                        ugli::uniforms! {
                            u_model_matrix: mat3::identity(),
                            u_color: Rgba::WHITE,
                            u_texture: &*mesh.texture,
                        },
                        self.camera.uniforms(self.framebuffer_size),
                    ),
                    ugli::DrawParameters::default(),
                );
            }
            for entity in &layer.entities {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::TexturedPolygon::new(
                        vec![
                            draw2d::TexturedVertex {
                                a_pos: vec2(0.0, 0.0),
                                a_color: Rgba::WHITE,
                                a_vt: entity.texture.uvs.bottom_left(),
                            },
                            draw2d::TexturedVertex {
                                a_pos: vec2(1.0, 0.0),
                                a_color: Rgba::WHITE,
                                a_vt: entity.texture.uvs.bottom_right(),
                            },
                            draw2d::TexturedVertex {
                                a_pos: vec2(1.0, 1.0),
                                a_color: Rgba::WHITE,
                                a_vt: entity.texture.uvs.top_right(),
                            },
                            draw2d::TexturedVertex {
                                a_pos: vec2(0.0, 1.0),
                                a_color: Rgba::WHITE,
                                a_vt: entity.texture.uvs.top_left(),
                            },
                        ],
                        &*entity.texture.atlas,
                    )
                    .translate(entity.pos.map(|x| x as f32)),
                );
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
        Game::new(&geng, &Rc::new(assets))
    });
}
