use super::*;

#[derive(Deserialize)]
pub struct Config {
    ui_fov: f32,
}

pub async fn confirm(ctx: &Context, title: &str) -> bool {
    let camera = geng::Camera2d {
        center: vec2::ZERO,
        rotation: Angle::ZERO,
        fov: 10.0,
    };
    enum ButtonType {
        Yes,
        No,
    }
    let mut buttons = [
        Button::square(Anchor::CENTER, vec2(-1, -1), ButtonType::Yes),
        Button::square(Anchor::CENTER, vec2(0, -1), ButtonType::No),
    ];
    let ui_camera = geng::Camera2d {
        center: vec2::ZERO,
        rotation: Angle::ZERO,
        fov: ctx.assets.config.popup.ui_fov,
    };
    let mut framebuffer_size = vec2::splat(1.0);
    let mut cursor_position = None;

    let mut events = ctx.geng.window().events();
    while let Some(event) = events.next().await {
        let check_click = |position: vec2<f64>| {
            let ui_pos = ui_camera.screen_to_world(framebuffer_size, position.map(|x| x as f32));
            if let Some(button) = buttons
                .iter()
                .find(|button| button.calculated_pos.contains(ui_pos))
            {
                match button.button_type {
                    ButtonType::Yes => return Some(true),
                    ButtonType::No => return Some(false),
                }
            }
            None
        };
        match event {
            geng::Event::KeyPress { key: geng::Key::Y } => {
                return true;
            }
            geng::Event::KeyPress {
                key: geng::Key::Escape | geng::Key::N,
            } => {
                return false;
            }
            geng::Event::CursorMove { position } => {
                cursor_position = Some(position);
            }
            geng::Event::MousePress { .. } => {
                if let Some(position) = cursor_position {
                    if let Some(result) = check_click(position) {
                        return result;
                    }
                }
            }
            geng::Event::TouchStart(geng::Touch { position, .. }) => {
                if let Some(result) = check_click(position) {
                    return result;
                }
            }
            geng::Event::Draw => {
                ctx.geng.window().with_framebuffer(|framebuffer| {
                    framebuffer_size = framebuffer.size().map(|x| x as f32);

                    ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

                    buttons::layout(
                        &mut buttons,
                        ui_camera.view_area(framebuffer_size).bounding_box(),
                    );
                    for (matrix, button) in buttons::matrices(
                        cursor_position.map(|pos| {
                            ui_camera.screen_to_world(framebuffer_size, pos.map(|x| x as f32))
                        }),
                        &buttons,
                    ) {
                        ctx.renderer.draw_game_tile(
                            framebuffer,
                            &ui_camera,
                            match button.button_type {
                                ButtonType::Yes => "Yes",
                                ButtonType::No => "No",
                            },
                            Rgba::WHITE,
                            matrix,
                        );
                    }
                    ctx.geng.default_font().draw(
                        framebuffer,
                        &camera,
                        title,
                        vec2(geng::TextAlign::CENTER, geng::TextAlign::LEFT),
                        mat3::translate(vec2(0.0, 1.5)) * mat3::scale_uniform(0.7),
                        Rgba::GRAY,
                    );
                    // ctx.geng.default_font().draw(
                    //     framebuffer,
                    //     &camera,
                    //     "Y/N",
                    //     vec2(geng::TextAlign::CENTER, geng::TextAlign::RIGHT),
                    //     mat3::identity(),
                    //     Rgba::WHITE,
                    // );
                });
            }
            _ => {}
        }
    }
    false
}

pub async fn prompt(ctx: &Context, title: &str, default_value: &str) -> Option<String> {
    let mut value = default_value.to_owned();
    let camera = geng::Camera2d {
        center: vec2::ZERO,
        rotation: Angle::ZERO,
        fov: 10.0,
    };

    // TODO move into engine
    struct TextEditGuard {
        window: geng::Window,
    }
    impl TextEditGuard {
        fn new(window: &geng::Window, value: &str) -> Self {
            window.start_text_edit(value);
            Self {
                window: window.clone(),
            }
        }
    }
    impl Drop for TextEditGuard {
        fn drop(&mut self) {
            self.window.stop_text_edit();
        }
    }
    let _guard = TextEditGuard::new(ctx.geng.window(), &value);

    let mut events = ctx.geng.window().events();
    while let Some(event) = events.next().await {
        match event {
            geng::Event::KeyPress {
                key: geng::Key::Enter,
            } => {
                return Some(value);
            }
            geng::Event::KeyPress {
                key: geng::Key::Escape,
            } => {
                break;
            }
            geng::Event::EditText(new_value) => value = new_value,
            geng::Event::Draw => {
                ctx.geng.window().with_framebuffer(|framebuffer| {
                    ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
                    ctx.geng.default_font().draw(
                        framebuffer,
                        &camera,
                        title,
                        vec2(geng::TextAlign::CENTER, geng::TextAlign::LEFT),
                        mat3::translate(vec2(0.0, 1.5)) * mat3::scale_uniform(0.7),
                        Rgba::GRAY,
                    );
                    ctx.geng.default_font().draw(
                        framebuffer,
                        &camera,
                        &value,
                        vec2(geng::TextAlign::CENTER, geng::TextAlign::RIGHT),
                        mat3::identity(),
                        Rgba::WHITE,
                    );
                });
            }
            _ => {}
        }
    }
    None
}
