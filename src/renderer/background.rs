use super::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    #[load(options(wrap_mode = "ugli::WrapMode::Repeat", premultiply_alpha = "true"))]
    top: ugli::Texture,
    #[load(options(wrap_mode = "ugli::WrapMode::Repeat", premultiply_alpha = "true"))]
    bottom: ugli::Texture,
}

pub struct State {
    assets: Rc<crate::Assets>,
    quad: ugli::VertexBuffer<draw2d::Vertex>,
}

impl State {
    pub fn new(geng: &Geng, assets: &Rc<crate::Assets>) -> Self {
        Self {
            assets: assets.clone(),
            quad: ugli::VertexBuffer::new_static(
                geng.ugli(),
                vec![
                    draw2d::Vertex {
                        a_pos: vec2(-1.0, -1.0),
                    },
                    draw2d::Vertex {
                        a_pos: vec2(1.0, -1.0),
                    },
                    draw2d::Vertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    draw2d::Vertex {
                        a_pos: vec2(-1.0, 1.0),
                    },
                ],
            ),
        }
    }

    pub fn draw(&self, framebuffer: &mut ugli::Framebuffer, camera: &impl geng::AbstractCamera2d) {
        let mut draw_layer = |texture: &ugli::Texture, k: f32| {
            ugli::draw(
                framebuffer,
                &self.assets.renderer.shaders.background,
                ugli::DrawMode::TriangleFan,
                &self.quad,
                (
                    ugli::uniforms! {
                        u_scale: texture.size().map(|x| x as f32) / self.assets.config.cell_pixel_size as f32,
                        u_parallax: vec2::splat(k),
                        u_texture: texture,
                        u_texture_size: texture.size(),
                    },
                    camera.uniforms(framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::premultiplied_alpha()),
                    ..default()
                },
            );
        };
        draw_layer(&self.assets.renderer.background.bottom, 0.75);
        draw_layer(&self.assets.renderer.background.top, 0.5);
    }
}
