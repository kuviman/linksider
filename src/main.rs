use geng::prelude::*;

mod camera_controls;
mod config;
mod editor;
mod history;
mod play;
mod renderer;
mod sound;
mod util;

use camera_controls::CameraControls;
use config::Config;
use logicsider::*;
use renderer::Renderer;
use util::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    pub config: Config,
    #[load(serde, path = "logic.toml")]
    pub logic_config: logicsider::Config,
    pub renderer: renderer::Assets,
    pub sound: sound::Assets,
}

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    editor: bool,
    #[clap(flatten)]
    geng: geng::CliArgs,
}

fn main() {
    logger::init();
    geng::setup_panic_handler();
    let cli_args: Opt = cli::parse();
    let geng = Geng::new_with(geng::ContextOptions {
        title: "LinkSider".to_owned(),
        ..geng::ContextOptions::from_args(&cli_args.geng)
    });
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
            level_count: usize,
            current_level: Cell<usize>,
            geng: Geng,
            assets: Rc<Assets>,
            sound: Rc<sound::State>,
            renderer: Rc<Renderer>,
        }

        impl LevelChanger {
            fn finisher(self: Rc<Self>) -> play::FinishCallback {
                Rc::new({
                    let state = self.clone();
                    move |finish| state.clone().finish(finish)
                })
            }

            fn load_game_state(&self) -> GameState {
                ron::de::from_reader(std::io::BufReader::new(
                    std::fs::File::open(self.level_path()).unwrap(),
                ))
                .unwrap()
            }

            fn level_path(&self) -> std::path::PathBuf {
                run_dir()
                    .join("assets")
                    .join("levels")
                    .join(format!("{}.ron", self.current_level.get()))
            }

            fn play(self: Rc<Self>) -> impl geng::State {
                play::State::new(
                    &self.geng,
                    &self.assets,
                    &self.renderer,
                    &self.sound,
                    self.load_game_state(),
                    self.clone().finisher(),
                )
            }

            fn editor(self: Rc<Self>) -> impl geng::State {
                editor::level::State::new(
                    &self.geng,
                    &self.assets,
                    &self.sound,
                    &self.renderer,
                    self.load_game_state(),
                    self.level_path(),
                    Some(self.clone().finisher()),
                )
            }

            fn finish(self: Rc<Self>, finish: play::Finish) -> geng::state::Transition {
                self.current_level.set({
                    let new_level = self.current_level.get() as isize
                        + match finish {
                            play::Finish::NextLevel => 1,
                            play::Finish::PrevLevel => -1,
                            play::Finish::Editor => 0,
                        };
                    new_level.clamp(0, self.level_count as isize - 1) as usize
                });
                geng::state::Transition::Switch(if let play::Finish::Editor = finish {
                    Box::new(self.editor())
                } else {
                    Box::new(self.play())
                })
            }
        }

        let level_count =
            file::load_string(run_dir().join("assets").join("levels").join("count.txt"))
                .await
                .unwrap()
                .trim()
                .parse()
                .unwrap();

        if cli_args.editor {
            Box::new(editor::world::State::load(geng, assets, &sound, &renderer))
                as Box<dyn geng::State>
        } else {
            Box::new(
                Rc::new(LevelChanger {
                    level_count,
                    current_level: Cell::new(0),
                    geng: geng.clone(),
                    assets: assets.clone(),
                    sound,
                    renderer,
                })
                .play(),
            )
        }
    });
}
