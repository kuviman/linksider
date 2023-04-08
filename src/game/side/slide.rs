use super::*;

pub fn init(app: &mut App) {
    app.register_side_effect::<Slide>("SlidePower");
    app.add_systems(
        (do_slide,)
            .in_set(OnUpdate(GameState::Turn))
            .before(end_turn),
    );
}

#[derive(Default, Component)]
pub struct Slide;

impl SideEffect for Slide {
    fn texture() -> &'static str {
        "side_effects/slide.png"
    }
}

fn do_slide() {}
