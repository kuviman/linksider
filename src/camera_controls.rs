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
    prev_drag_pos: Option<vec2<f64>>,
}

impl CameraControls {
    pub fn new(geng: &Geng, config: &Rc<Config>) -> Self {
        Self {
            geng: geng.clone(),
            config: config.clone(),
            prev_drag_pos: None,
        }
    }
    pub fn handle_event(&mut self, camera: &mut geng::Camera2d, event: geng::Event) -> bool {
        let framebuffer_size = self.geng.window().size().map(|x| x as f32);
        match event {
            geng::Event::MouseDown { position, button } if button == self.config.controls.drag => {
                self.prev_drag_pos = Some(position);
                return true;
            }
            geng::Event::MouseUp { button, .. } if button == self.config.controls.drag => {
                self.prev_drag_pos = None;
                return true;
            }
            geng::Event::MouseMove { position, .. } => {
                if let Some(drag) = &mut self.prev_drag_pos {
                    let world_pos = |pos: vec2<f64>| -> vec2<f32> {
                        camera.screen_to_world(framebuffer_size, pos.map(|x| x as f32))
                    };
                    let before = world_pos(*drag);
                    let now = world_pos(position);
                    camera.center += before - now;
                    self.prev_drag_pos = Some(position);
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
