use super::*;

#[derive(Deserialize)]
pub struct Config {
    ui_fov: f32,
}

pub async fn confirm(ctx: &Context, actx: &mut async_states::Context, title: &str) -> bool {
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
    loop {
        match actx.wait().await {
            async_states::Event::Update(_) => {}
            async_states::Event::Event(event) => match event {
                geng::Event::KeyDown { key: geng::Key::Y } => {
                    return true;
                }
                geng::Event::KeyDown {
                    key: geng::Key::Escape | geng::Key::N,
                } => {
                    return false;
                }
                geng::Event::MouseDown { position, .. }
                | geng::Event::TouchStart(geng::Touch { position, .. }) => {
                    let ui_pos =
                        ui_camera.screen_to_world(framebuffer_size, position.map(|x| x as f32));
                    if let Some(button) = buttons
                        .iter()
                        .find(|button| button.calculated_pos.contains(ui_pos))
                    {
                        match button.button_type {
                            ButtonType::Yes => return true,
                            ButtonType::No => return false,
                        }
                    }
                }
                _ => {}
            },
            async_states::Event::Draw => {
                let mut framebuffer = actx.framebuffer();
                let framebuffer: &mut ugli::Framebuffer = &mut framebuffer;
                framebuffer_size = framebuffer.size().map(|x| x as f32);

                ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

                buttons::layout(
                    &mut buttons,
                    ui_camera.view_area(framebuffer_size).bounding_box(),
                );
                for (matrix, button) in buttons::matrices(
                    ui_camera.screen_to_world(
                        framebuffer_size,
                        ctx.geng.window().cursor_position().map(|x| x as f32),
                    ),
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
            }
        }
    }
}

pub async fn prompt(
    ctx: &Context,
    actx: &mut async_states::Context,
    title: &str,
    default_value: &str,
) -> Option<String> {
    let mut value = default_value.to_owned();
    let camera = geng::Camera2d {
        center: vec2::ZERO,
        rotation: Angle::ZERO,
        fov: 10.0,
    };
    ctx.geng.window().start_text_edit(&value);
    loop {
        match actx.wait().await {
            async_states::Event::Update(_) => {}
            async_states::Event::Event(event) => match event {
                geng::Event::KeyDown {
                    key: geng::Key::Enter,
                } => {
                    ctx.geng.window().stop_text_edit();
                    return Some(value);
                }
                geng::Event::KeyDown {
                    key: geng::Key::Escape,
                } => {
                    ctx.geng.window().stop_text_edit();
                    return None;
                }
                geng::Event::EditText(new_value) => value = new_value,
                _ => {}
            },
            async_states::Event::Draw => {
                let mut framebuffer = actx.framebuffer();
                let framebuffer: &mut ugli::Framebuffer = &mut framebuffer;

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
            }
        }
    }
}
