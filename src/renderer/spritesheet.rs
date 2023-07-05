use super::*;

pub struct SpriteSheet {
    texture: ugli::Texture,
}

impl geng::asset::Load for SpriteSheet {
    type Options = ();
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        _options: &Self::Options,
    ) -> geng::asset::Future<Self> {
        manager
            .load_with::<ugli::Texture>(
                path,
                &geng::asset::TextureOptions {
                    premultiply_alpha: true,
                    ..default()
                },
            )
            .map_ok(|texture| Self { texture })
            .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

impl Renderer {
    pub fn draw_sprite_sheet(
        &self,
        sprite_sheet: &SpriteSheet,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        color: Rgba<f32>,
        t: f32,
        transform: mat3<f32>,
    ) {
        assert!(sprite_sheet.texture.size().y % sprite_sheet.texture.size().x == 0);
        let frames = sprite_sheet.texture.size().y / sprite_sheet.texture.size().x;
        let frame = ((t - t.floor()) * frames as f32).floor();
        let start_vt = frame / frames as f32;
        let end_vt = (frame + 1.0) / frames as f32;
        let (start_vt, end_vt) = (1.0 - end_vt, 1.0 - start_vt);
        let uv_aabb = Aabb2 {
            min: vec2(0.0, start_vt),
            max: vec2(1.0, end_vt),
        };
        let v = |x, y| TilesetVertex {
            a_pos: vec2(x as f32, y as f32),
            a_uv: uv_aabb.bottom_left() + uv_aabb.size() * vec2(x as f32, y as f32),
        };
        // TODO
        let vertex_data = ugli::VertexBuffer::new_dynamic(
            self.geng.ugli(),
            vec![v(0, 0), v(1, 0), v(1, 1), v(0, 1)],
        );
        self.draw_mesh_impl(
            framebuffer,
            camera,
            &vertex_data,
            ugli::DrawMode::TriangleFan,
            &sprite_sheet.texture,
            color,
            transform
                * mat3::scale_uniform(
                    sprite_sheet.texture.size().x as f32
                        / self.assets.config.cell_pixel_size as f32,
                )
                * mat3::translate(vec2::splat(-0.5)),
        );
    }
}
