use super::*;

#[derive(Deref, DerefMut)]
pub struct Texture(#[deref] ugli::Texture);

impl geng::asset::Load for Texture {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        let texture = ugli::Texture::load(manager, path);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Self(texture))
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

pub fn process_texture(texture: &mut ugli::Texture) {
    texture.set_filter(ugli::Filter::Nearest);
    texture.set_wrap_mode(ugli::WrapMode::Repeat);
}

// TODO move into batbox
pub fn lerp<V, T>(a: V, b: V, t: T) -> V
where
    V: Mul<T, Output = V> + Add<Output = V>,
    T: Float,
{
    a * (T::ONE - t) + b * t
}
