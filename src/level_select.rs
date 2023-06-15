use super::*;

#[derive(Deserialize)]
pub struct Config {
    margin: f32,
    fov: f32,
    ui_fov: f32,
    level_icon_size: f32,
}

pub async fn run(ctx: &Context, actx: &mut async_states::Context) {
    let config = &ctx.assets.config.level_select;
    State {
        ctx: ctx.clone(),
        framebuffer_size: vec2::splat(1.0),
        config: config.clone(),
        groups: future::join_all(levels::load_group_names().await.into_iter().map(
            |group_name| async move {
                Group {
                    levels: levels::load_level_names(&group_name)
                        .await
                        .into_iter()
                        .map(|level_name| Level { name: level_name })
                        .collect(),
                    name: group_name,
                }
            },
        ))
        .await,
        camera: geng::Camera2d {
            center: vec2::ZERO,
            rotation: Angle::ZERO,
            fov: config.fov,
        },
        ui_camera: geng::Camera2d {
            center: vec2::ZERO,
            rotation: Angle::ZERO,
            fov: config.ui_fov,
        },
        input: input::State::new(ctx),
        buttons: Box::new([Button::square(
            Anchor::TOP_RIGHT,
            vec2(-1, -1),
            ButtonType::Editor,
        )]),
    }
    .run(actx)
    .await
}

struct Level {
    name: String,
}

struct Group {
    name: String,
    levels: Vec<Level>,
}

struct Selection {
    group: usize,
    level: usize,
}

pub struct State {
    ctx: Context,
    framebuffer_size: vec2<f32>,
    config: Rc<Config>,
    groups: Vec<Group>,
    camera: geng::Camera2d,
    ui_camera: geng::Camera2d,
    input: input::State,
    buttons: Box<[Button<ButtonType>]>,
}

enum ButtonType {
    Editor,
}

fn level_screen_pos(group_index: usize, level_index: usize) -> vec2<i32> {
    vec2(level_index as i32, -(group_index as i32))
}

impl State {
    fn clamp_camera(&mut self) {
        let aabb = Aabb2::points_bounding_box(self.groups.iter().enumerate().flat_map(
            |(group_index, group)| {
                group
                    .levels
                    .iter()
                    .enumerate()
                    .map(move |(level_index, _level)| level_screen_pos(group_index, level_index))
            },
        ))
        .unwrap()
        .extend_positive(vec2::splat(1))
        .map(|x| x as f32)
        .extend_uniform(self.config.margin);
        self.camera.center = self.camera.center.clamp_aabb({
            let mut aabb = aabb.extend_symmetric(
                -vec2(self.framebuffer_size.aspect(), 1.0) * self.camera.fov / 2.0,
            );
            if aabb.min.x > aabb.max.x {
                let center = (aabb.min.x + aabb.max.x) / 2.0;
                aabb.min.x = center;
                aabb.max.x = center;
            }
            if aabb.min.y > aabb.max.y {
                let center = (aabb.min.y + aabb.max.y) / 2.0;
                aabb.min.y = center;
                aabb.max.y = center;
            }
            aabb
        });
    }

    fn hovered(&self, screen_pos: vec2<f64>) -> Option<Selection> {
        let world_pos = self.camera.screen_to_world(
            self.ctx.geng.window().size().map(|x| x as f32),
            screen_pos.map(|x| x as f32),
        );
        let places = self
            .groups
            .iter()
            .enumerate()
            .flat_map(|(group_index, group)| {
                group
                    .levels
                    .iter()
                    .enumerate()
                    .map(move |(level_index, _level)| (group_index, level_index))
            });
        for (group_index, level_index) in places {
            if Aabb2::point(level_screen_pos(group_index, level_index))
                .extend_positive(vec2::splat(1))
                .map(|x| x as f32)
                .contains(world_pos)
            {
                return Some(Selection {
                    group: group_index,
                    level: level_index,
                });
            }
        }
        None
    }

    async fn play_impl(
        &mut self,
        actx: &mut async_states::Context,
        selection: Selection,
    ) -> Option<Selection> {
        let group = &self.groups[selection.group];
        let level = &group.levels[selection.level];
        let mut level_state =
            logicsider::Level::load_from_file(levels::level_path(&group.name, &level.name))
                .await
                .unwrap();
        let finish = play::State::new(&self.ctx, &level_state).run(actx).await;
        let mut selection = selection;
        match finish {
            play::Transition::NextLevel => {
                if selection.level + 1 < group.levels.len() {
                    selection.level += 1;
                    return Some(selection);
                }
            }
            play::Transition::PrevLevel => {
                if selection.level > 0 {
                    selection.level -= 1;
                    return Some(selection);
                }
            }
            play::Transition::Editor => {
                editor::level::State::new(
                    &self.ctx,
                    format!("{}::{}", group.name, level.name),
                    &mut level_state,
                    levels::level_path(&group.name, &level.name),
                )
                .run(actx)
                .await
            }
            play::Transition::Exit => {}
        }
        None
    }

    async fn play(&mut self, actx: &mut async_states::Context, selection: Selection) {
        let mut selection = Some(selection);
        while let Some(current) = selection {
            selection = self.play_impl(actx, current).await;
        }
    }
}

impl input::Context for State {
    fn input(&mut self) -> &mut input::State {
        &mut self.input
    }
    fn is_draggable(&self, _screen_pos: vec2<f64>) -> bool {
        false
    }
}

impl State {
    async fn run(mut self, actx: &mut async_states::Context) {
        loop {
            match actx.wait().await {
                async_states::Event::Event(event) => self.handle_event(actx, event).await,
                async_states::Event::Update(delta_time) => self.update(actx, delta_time).await,
                async_states::Event::Draw => self.draw(&mut actx.framebuffer()),
            }
        }
    }
    async fn update(&mut self, actx: &mut async_states::Context, delta_time: f64) {
        for event in input::Context::update(self, delta_time) {
            self.handle_input(actx, event).await;
        }
    }
    async fn handle_event(&mut self, actx: &mut async_states::Context, event: geng::Event) {
        for event in input::Context::handle_event(self, event.clone()) {
            self.handle_input(actx, event).await;
        }
    }
    async fn handle_input(&mut self, actx: &mut async_states::Context, event: input::Event) {
        match event {
            input::Event::Click(position) => {
                let ui_pos = self
                    .ui_camera
                    .screen_to_world(self.framebuffer_size, position.map(|x| x as f32));
                if let Some(button) = self
                    .buttons
                    .iter()
                    .find(|button| button.calculated_pos.contains(ui_pos))
                {
                    match button.button_type {
                        ButtonType::Editor => {
                            editor::world::State::load(&self.ctx, actx).await;
                        }
                    }
                } else if let Some(selection) = self.hovered(position) {
                    self.play(actx, selection).await;
                }
            }
            input::Event::TransformView(transform) => {
                // TODO not allow transforms?
                transform.apply(&mut self.camera, self.framebuffer_size);
                self.camera.rotation = Angle::ZERO;
            }
            _ => unreachable!(),
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.clamp_camera();
        self.ctx.renderer.draw_background(framebuffer, &self.camera);
        for (group_index, group) in self.groups.iter().enumerate() {
            for (level_index, _level) in group.levels.iter().enumerate() {
                self.ctx.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::Quad::new(
                        Aabb2::point(
                            level_screen_pos(group_index, level_index).map(|x| x as f32 + 0.5),
                        )
                        .extend_symmetric(vec2::splat(self.config.level_icon_size / 2.0)),
                        Rgba::WHITE,
                    ),
                );
            }
        }
        if let Some(selection) = self.hovered(self.ctx.geng.window().cursor_position()) {
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.camera,
                "EditorSelect",
                Rgba::WHITE,
                mat3::translate(
                    level_screen_pos(selection.group, selection.level).map(|x| x as f32),
                ),
            );
            let group = &self.groups[selection.group];
            let level = &group.levels[selection.level];
            let text = format!("{}::{}", group.name, level.name);
            self.ctx.geng.default_font().draw_with_outline(
                framebuffer,
                &self.camera,
                &text,
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(
                    level_screen_pos(selection.group, selection.level).map(|x| x as f32 + 0.5)
                        + vec2(0.0, 1.0),
                ),
                Rgba::WHITE,
                0.05,
                Rgba::BLACK,
            );
        }

        buttons::layout(
            &mut self.buttons,
            self.ui_camera
                .view_area(self.framebuffer_size)
                .bounding_box(),
        );
        let ui_cursor_pos = self.ui_camera.screen_to_world(
            self.framebuffer_size,
            self.ctx.geng.window().cursor_position().map(|x| x as f32),
        );
        for (matrix, button) in buttons::matrices(ui_cursor_pos, &self.buttons) {
            self.ctx.renderer.draw_tile(
                framebuffer,
                &self.ui_camera,
                match button.button_type {
                    ButtonType::Editor => "Edit",
                },
                Rgba::WHITE,
                matrix,
            );
        }
    }
}
