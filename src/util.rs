use super::*;

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
