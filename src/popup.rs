use super::*;

pub async fn prompt(
    ctx: &Context,
    actx: &mut async_states::Context,
    title: &str,
    default_value: &str,
) -> Option<String> {
    let mut value = default_value.to_owned();
    let camera = geng::Camera2d {
        center: vec2::ZERO,
        rotation: 0.0,
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
                    vec2::splat(geng::TextAlign::CENTER),
                    mat3::translate(vec2(0.0, 1.5)) * mat3::scale_uniform(0.7),
                    Rgba::GRAY,
                );
                ctx.geng.default_font().draw(
                    framebuffer,
                    &camera,
                    &value,
                    vec2::splat(geng::TextAlign::CENTER),
                    mat3::identity(),
                    Rgba::WHITE,
                );
            }
        }
    }
}
