use super::*;

#[derive(Deserialize)]
pub struct Controls {
    drag: geng::MouseButton,
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(flatten)]
    controls: Controls,
    zoom_speed: f32,
    min_fov: f32,
    max_fov: f32,
}

pub struct CameraControls {
    geng: Geng,
    config: Rc<Config>,
    prev_drag_pos: Option<vec2<f32>>,
}

impl CameraControls {
    pub fn new(geng: &Geng, config: &Rc<Config>) -> Self {
        Self {
            geng: geng.clone(),
            config: config.clone(),
            prev_drag_pos: None,
        }
    }
    pub fn start_drag(&mut self, world_pos: vec2<f32>) {
        self.prev_drag_pos = Some(world_pos);
    }
    pub fn end_drag(&mut self) -> bool {
        self.prev_drag_pos.take().is_some()
    }
    pub fn move_drag(&mut self, camera: &mut geng::Camera2d, world_pos: vec2<f32>) -> bool {
        let Some(before) = self.prev_drag_pos else { return false };
        camera.center += before - world_pos;
        self.prev_drag_pos = Some(world_pos);
        true
    }

    #[deprecated]
    pub fn handle_event(&mut self, camera: &mut geng::Camera2d, event: geng::Event) -> bool {
        let framebuffer_size = self.geng.window().size().map(|x| x as f32);
        let world_pos = |pos: vec2<f64>| -> vec2<f32> {
            camera.screen_to_world(framebuffer_size, pos.map(|x| x as f32))
        };
        match event {
            geng::Event::MouseDown { position, button } => {
                if button == self.config.controls.drag {
                    self.start_drag(world_pos(position));
                    return true;
                }
            }
            geng::Event::MouseUp { .. } if self.prev_drag_pos.is_some() => {
                self.end_drag();
                return true;
            }
            geng::Event::MouseMove { position, .. } => {
                if self.prev_drag_pos.is_some() {
                    self.move_drag(camera, world_pos(position));
                    return true;
                }
            }
            geng::Event::Wheel { delta } => {
                let before = camera.screen_to_world(
                    framebuffer_size,
                    self.geng.window().cursor_position().map(|x| x as f32),
                );
                camera.fov = (camera.fov - delta as f32 * self.config.zoom_speed)
                    .clamp(self.config.min_fov, self.config.max_fov);
                let now = camera.screen_to_world(
                    framebuffer_size,
                    self.geng.window().cursor_position().map(|x| x as f32),
                );
                camera.center += before - now;
                return true;
            }
            _ => {}
        }
        false
    }
}
