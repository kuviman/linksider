use geng::prelude::*;

use ldtk::Ldtk;

mod config;
mod editor;
mod history;
mod play;
mod renderer;
mod sound;
mod util;

use config::Config;
use logicsider::*;
use renderer::Renderer;
use util::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    pub config: Config,
    #[load(serde, path = "logic.toml")]
    pub logic_config: logicsider::Config,
    pub world: Ldtk,
    pub renderer: renderer::Assets,
    pub sound: sound::Assets,
}

fn main() {
    logger::init();
    geng::setup_panic_handler();
    let geng = Geng::new("linksider");
    geng.clone().run_loading(async move {
        let geng = &geng;
        let assets: Assets = geng
            .asset_manager()
            .load(run_dir().join("assets"))
            .await
            .unwrap();
        let assets = Rc::new(assets);
        let assets = &assets;
        let sound = Rc::new(sound::State::new(geng, assets));
        let renderer = Rc::new(Renderer::new(geng, assets));

        struct LevelChanger {
            current_level: Cell<usize>,
            geng: Geng,
            assets: Rc<Assets>,
            sound: Rc<sound::State>,
            renderer: Rc<Renderer>,
        }

        impl LevelChanger {
            fn play(self: Rc<Self>) -> impl geng::State {
                play::State::new(
                    &self.geng,
                    &self.assets,
                    &self.renderer,
                    &self.sound,
                    GameState::from_ldtk(
                        &self.assets.world.json,
                        &self.assets.logic_config,
                        self.current_level.get(),
                    ),
                    Rc::new({
                        let state = self.clone();
                        move |finish| state.clone().finish(finish)
                    }),
                )
            }
            fn finish(self: Rc<Self>, finish: play::Finish) -> geng::state::Transition {
                self.current_level.set({
                    let new_level = self.current_level.get() as isize
                        + match finish {
                            play::Finish::NextLevel => 1,
                            play::Finish::PrevLevel => -1,
                        };
                    new_level.clamp(0, self.assets.world.levels.len() as isize - 1) as usize
                });
                geng::state::Transition::Switch(Box::new(self.play()))
            }
        }

        Rc::new(LevelChanger {
            current_level: Cell::new(0),
            geng: geng.clone(),
            assets: assets.clone(),
            sound,
            renderer,
        })
        .play()
    });
}
