use super::*;

#[derive(Deserialize)]
struct TextureConfig {
    anchor: Anchor,
    texture: std::path::PathBuf,
}

#[derive(Deserialize)]
struct Config {
    textures: Vec<TextureConfig>,
}

pub struct Assets {
    textures: HashMap<Anchor, ugli::Texture>,
}

// ACTIVATING LINUX IS FOR NOOBS
impl geng::asset::Load for Assets {
    type Options = ();
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        _options: &Self::Options,
    ) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let config: Config = file::load_detect(path.join("config.toml")).await?;
            let manager = &manager;
            let path = &path;
            Ok(Assets {
                textures: future::join_all(config.textures.into_iter().map(|config| async move {
                    Ok::<_, anyhow::Error>((
                        config.anchor,
                        manager
                            .load_with(
                                path.join(config.texture),
                                &geng::asset::TextureOptions {
                                    // premultiply_alpha: true,
                                    ..default()
                                },
                            )
                            .await?,
                    ))
                }))
                .await
                .into_iter()
                .collect::<Result<HashMap<_, _>, _>>()?,
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = None;
}

impl Renderer {
    pub fn draw_ui_background(
        &self,
        assets: &Assets,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
    ) {
        let viewport = camera
            .view_area(framebuffer.size().map(|x| x as f32))
            .bounding_box();
        for (&anchor, texture) in &assets.textures {
            let size = texture.size().map(|x| x as f32)
                / self.assets.renderer.ui.def.tile_size.map(|x| x as f32);
            self.geng.draw2d().draw2d(
                framebuffer,
                camera,
                &draw2d::TexturedQuad::new(
                    Aabb2::point(viewport.bottom_left() + (viewport.size() - size) * anchor.v())
                        .extend_positive(size),
                    texture,
                ),
            );
        }
    }
}
