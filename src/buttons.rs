use super::*;

pub fn matrices<T>(
    cursor_pos: vec2<f32>,
    buttons: &[Button<T>],
) -> impl Iterator<Item = (mat3<f32>, &Button<T>)> + '_ {
    buttons.iter().map(move |button| {
        let matrix = mat3::translate(button.calculated_pos.bottom_left())
            * mat3::scale(button.calculated_pos.size())
            * mat3::scale_uniform_around(
                vec2::splat(0.5),
                if button.usable && button.calculated_pos.contains(cursor_pos) {
                    1.1
                } else {
                    1.0
                },
            );
        (matrix, button)
    })
}

pub fn layout<T>(buttons: &mut [Button<T>], viewport: Aabb2<f32>) {
    for button in buttons {
        button.calculated_pos = button
            .pos
            .translate(viewport.bottom_left() + viewport.size() * button.anchor.0);
    }
}

#[derive(Copy, Clone)]
pub struct Anchor(vec2<f32>);

impl Anchor {
    pub const TOP_LEFT: Self = Self(vec2(0.0, 1.0));
    pub const TOP_RIGHT: Self = Self(vec2(1.0, 1.0));
    pub const BOTTOM_LEFT: Self = Self(vec2(0.0, 0.0));
    pub const BOTTOM_RIGHT: Self = Self(vec2(1.0, 0.0));
}

pub struct Button<T> {
    pub usable: bool,
    pub anchor: Anchor,
    pub pos: Aabb2<f32>,
    pub calculated_pos: Aabb2<f32>,
    pub button_type: T,
}

impl<T> Button<T> {
    pub fn new(anchor: Anchor, pos: Aabb2<f32>, button_type: T) -> Self {
        Self {
            anchor,
            pos,
            button_type,
            calculated_pos: pos,
            usable: true,
        }
    }

    pub fn square(anchor: Anchor, pos: vec2<i32>, button_type: T) -> Self {
        // TODO configurable?
        let size = 1.0;
        let padding = 0.1;
        Self::new(
            anchor,
            Aabb2::point(pos.zip(anchor.0).map(|(x, anchor)| {
                (x as f32 * (size + padding) + padding + size / 2.0) * (1.0 - anchor * 2.0)
            }))
            .extend_symmetric(vec2::splat(size / 2.0)),
            button_type,
        )
    }
}
