use batbox::prelude::*;

// TODO move into batbox
pub fn lerp<V, T>(a: V, b: V, t: T) -> V
where
    V: Mul<T, Output = V> + Add<Output = V>,
    T: Float,
{
    a * (T::ONE - t) + b * t
}
