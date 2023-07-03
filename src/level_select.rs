use super::*;

#[derive(Deserialize)]
pub struct Controls {
    up: Vec<geng::Key>,
    down: Vec<geng::Key>,
    left: Vec<geng::Key>,
    right: Vec<geng::Key>,
    play: Vec<geng::Key>,
}

#[derive(Deserialize)]
pub struct Config {
    fov: f32,
    ui_fov: f32,
    connector_width: f32,
    connector_color: Rgba<f32>,
    icon_color: Rgba<f32>,
    level_icon_offset: vec2<f32>,
    level_icon_size: f32,
    camera_speed: f32,
    other_group_opacity: f32,
    select_scale: f32,
    controls: Controls,
}

pub async fn run(ctx: &Context) {
    let config = &ctx.assets.config.level_select;
    State {
        cursor_position: None,
        selection: Selection { group: 0, level: 0 },
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
        input: input::Controller::new(ctx),
        buttons: Box::new([Button::square(
            Anchor::TOP_RIGHT,
            vec2(-1, -1),
            ButtonType::Editor,
        )]),
    }
    .run()
    .await
}

struct Level {
    name: String,
}

struct Group {
    name: String,
    levels: Vec<Level>,
}

#[derive(Copy, Clone)]
struct Selection {
    group: usize,
    level: usize,
}

impl Selection {
    fn world_pos(&self) -> vec2<i32> {
        vec2(
            self.level as i32 + self.group as i32,
            -(self.group as i32 * 2),
        )
    }
}

pub struct State {
    ctx: Context,
    framebuffer_size: vec2<f32>,
    config: Rc<Config>,
    groups: Vec<Group>,
    selection: Selection,
    camera: geng::Camera2d,
    ui_camera: geng::Camera2d,
    cursor_position: Option<vec2<f64>>,
    input: input::Controller,
    buttons: Box<[Button<ButtonType>]>,
}

enum ButtonType {
    Editor,
}

impl State {
    fn clamp_camera(&mut self) {
        let mut min_y = Selection { group: 0, level: 0 }.world_pos().y as f32 + 0.5;
        let mut max_y = Selection {
            group: self.groups.len() - 1,
            level: 0,
        }
        .world_pos()
        .y as f32
            + 0.5;
        if min_y > max_y {
            mem::swap(&mut min_y, &mut max_y);
        }
        self.camera.center.y = self.camera.center.y.clamp(min_y, max_y);
        let group_index_f32 = -(self.camera.center.y - 0.5) / 2.0;
        let min_x = group_index_f32 + 0.5;
        let max_x = min_x + 9.0;
        self.camera.center.x = self.camera.center.x.clamp(min_x, max_x);
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
                    .map(move |(level_index, _level)| Selection {
                        group: group_index,
                        level: level_index,
                    })
            });
        for place in places {
            if Aabb2::point(place.world_pos())
                .extend_positive(vec2::splat(1))
                .map(|x| x as f32)
                .contains(world_pos)
            {
                return Some(place);
            }
        }
        None
    }

    async fn play_impl(&mut self, selection: Selection) -> Option<Selection> {
        let group = &self.groups[selection.group];
        let level = &group.levels[selection.level];
        let mut level_state =
            logicsider::Level::load_from_file(levels::level_path(&group.name, &level.name))
                .await
                .unwrap();
        let finish = play::State::new(&self.ctx, &level_state).run().await;
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
                .run()
                .await
            }
            play::Transition::Exit => {}
        }
        None
    }

    async fn play(&mut self, selection: Selection) {
        let mut selection = Some(selection);
        while let Some(current) = selection {
            selection = self.play_impl(current).await;
        }
    }

    fn viewed_level(&self) -> Selection {
        let group_index = (0..self.groups.len())
            .min_by_key(|&group_index| {
                r32((Selection {
                    group: group_index,
                    level: 0,
                }
                .world_pos()
                .y as f32
                    + 0.5
                    - self.camera.center.y)
                    .abs())
            })
            .unwrap();
        let level_index = (0..self.groups[group_index].levels.len())
            .min_by_key(|&level_index| {
                r32((Selection {
                    group: group_index,
                    level: level_index,
                }
                .world_pos()
                .x as f32
                    + 0.5
                    - self.camera.center.x)
                    .abs())
            })
            .unwrap();
        Selection {
            group: group_index,
            level: level_index,
        }
    }
}

impl input::Context for State {
    fn input(&mut self) -> &mut input::Controller {
        &mut self.input
    }
    fn is_draggable(&self, _screen_pos: vec2<f64>) -> bool {
        false
    }
}

impl State {
    async fn run(mut self) {
        let mut timer = Timer::new();
        let mut events = self.ctx.geng.window().events();
        while let Some(event) = events.next().await {
            match event {
                geng::Event::Draw => {
                    self.update(timer.tick().as_secs_f64()).await;
                    self.ctx
                        .geng
                        .window()
                        .clone()
                        .with_framebuffer(|framebuffer| {
                            self.draw(framebuffer);
                        });
                }
                _ => self.handle_event(event).await,
            }
        }
    }
    async fn update(&mut self, delta_time: f64) {
        for event in input::Context::update(self, delta_time) {
            self.handle_input(event).await;
        }
        let delta_time = delta_time as f32;
        if !matches!(self.input.state(), input::State::TransformView) {
            self.camera.center = lerp(
                self.camera.center,
                self.selection.world_pos().map(|x| x as f32 + 0.5),
                (delta_time * self.config.camera_speed).min(1.0),
            );
        }
    }
    async fn handle_event(&mut self, event: geng::Event) {
        for event in input::Context::handle_event(self, event.clone()) {
            self.handle_input(event).await;
        }
        if let geng::Event::CursorMove { position, .. } = event {
            self.cursor_position = Some(position);
        }
        if let geng::Event::KeyRelease { key } = event {
            if self.config.controls.left.contains(&key) {
                if self.selection.level > 0 {
                    self.selection.level -= 1;
                }
                self.cursor_position = None;
            }
            if self.config.controls.right.contains(&key) {
                if self.selection.level + 1 < self.groups[self.selection.group].levels.len() {
                    self.selection.level += 1;
                }
                self.cursor_position = None;
            }
            if self.config.controls.up.contains(&key) {
                if self.selection.group > 0 {
                    self.selection.group -= 1;
                    self.selection.level += 1;
                }
                self.cursor_position = None;
            }
            if self.config.controls.down.contains(&key) {
                if self.selection.group + 1 < self.groups.len() {
                    self.selection.group += 1;
                    if self.selection.level > 0 {
                        self.selection.level -= 1;
                    }
                }
                self.cursor_position = None;
            }
            self.selection.level = self
                .selection
                .level
                .min(self.groups[self.selection.group].levels.len() - 1);
            if self.config.controls.play.contains(&key) {
                self.play(self.selection).await;
            }
        }
    }
    async fn handle_input(&mut self, event: input::Event) {
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
                            editor::world::State::load(&self.ctx).await;
                        }
                    }
                } else if let Some(selection) = self.hovered(position) {
                    self.play(selection).await;
                }
            }
            input::Event::TransformView(transform) => {
                transform.apply(&mut self.camera, self.framebuffer_size);
                self.camera.fov = self.config.fov;
                self.camera.rotation = Angle::ZERO;
            }
            input::Event::StopTransformView => {
                self.selection = self.viewed_level();
                self.cursor_position = None;
            }
            _ => unreachable!(),
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.clamp_camera();

        let selection = if matches!(self.input.state(), input::State::TransformView) {
            self.viewed_level()
        } else {
            self.cursor_position
                .and_then(|pos| self.hovered(pos))
                .unwrap_or(self.selection)
        };

        self.ctx.renderer.draw_background(framebuffer, &self.camera);
        // TODO not create this texture every frame KEKW
        let mut other_groups =
            ugli::Texture::new_uninitialized(self.ctx.geng.ugli(), framebuffer.size());
        {
            let mut other_framebuffer = ugli::Framebuffer::new_color(
                self.ctx.geng.ugli(),
                ugli::ColorAttachment::Texture(&mut other_groups),
            );
            for (group_index, group) in self.groups.iter().enumerate() {
                let draw_group = |framebuffer: &mut ugli::Framebuffer| {
                    self.ctx.renderer.draw_group_icon(
                        framebuffer,
                        &self.camera,
                        &group.name,
                        Rgba::WHITE,
                        mat3::translate(
                            Selection {
                                group: group_index,
                                level: 0,
                            }
                            .world_pos()
                            .map(|x| x as f32)
                                + vec2(0.0, 1.0),
                        ),
                    );
                    self.ctx.geng.draw2d().draw2d(
                        framebuffer,
                        &self.camera,
                        &draw2d::Quad::new(
                            Aabb2::point(
                                Selection {
                                    group: group_index,
                                    level: 0,
                                }
                                .world_pos()
                                .map(|x| x as f32 + 0.5),
                            )
                            .extend_positive(vec2(group.levels.len() as f32 - 1.0, 0.0))
                            .extend_uniform(self.config.connector_width / 2.0),
                            self.config.connector_color,
                        ),
                    );
                    for (level_index, _level) in group.levels.iter().enumerate() {
                        let matrix = mat3::translate(
                            Selection {
                                group: group_index,
                                level: level_index,
                            }
                            .world_pos()
                            .map(|x| x as f32),
                        );
                        self.ctx.renderer.draw_ui_tile(
                            framebuffer,
                            &self.camera,
                            "LevelButton",
                            Rgba::WHITE,
                            matrix,
                        );
                        self.ctx.renderer.draw_index(
                            framebuffer,
                            &self.camera,
                            level_index,
                            self.config.icon_color,
                            matrix
                                * mat3::translate(self.config.level_icon_offset)
                                * mat3::scale_uniform_around(
                                    vec2::splat(0.5),
                                    self.config.level_icon_size,
                                ),
                        );
                    }
                };
                if group_index == selection.group {
                    draw_group(framebuffer);
                } else {
                    draw_group(&mut other_framebuffer);
                }
            }
        }
        self.ctx.geng.draw2d().draw2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw2d::TexturedQuad::colored(
                Aabb2::ZERO.extend_positive(self.framebuffer_size),
                &other_groups,
                Rgba::new(1.0, 1.0, 1.0, self.config.other_group_opacity),
            ),
        );
        self.ctx.renderer.draw_ui_tile(
            framebuffer,
            &self.camera,
            "Select",
            Rgba::WHITE,
            mat3::translate(selection.world_pos().map(|x| x as f32))
                * mat3::scale_uniform_around(vec2::splat(0.5), self.config.select_scale),
        );
        let group = &self.groups[selection.group];
        let _level = &group.levels[selection.level];

        buttons::layout(
            &mut self.buttons,
            self.ui_camera
                .view_area(self.framebuffer_size)
                .bounding_box(),
        );
        let ui_cursor_pos = self.cursor_position.map(|pos| {
            self.ui_camera
                .screen_to_world(self.framebuffer_size, pos.map(|x| x as f32))
        });
        for (matrix, button) in buttons::matrices(ui_cursor_pos, &self.buttons) {
            self.ctx.renderer.draw_game_tile(
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
