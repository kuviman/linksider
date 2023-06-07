use geng::prelude::*;

mod async_states;
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

#[derive(Clone)]
pub struct Context {
    geng: Geng,
    assets: Rc<Assets>,
    sound: Rc<sound::State>,
    renderer: Rc<Renderer>,
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
        let ctx = Rc::new(Context {
            geng: geng.clone(),
            assets: assets.clone(),
            sound,
            renderer,
        });

        Box::new(async_states::as_state(geng, |mut actx| async move {
            // TODO only run editor on --editor
            editor::world::State::load(&ctx, &mut actx).await;
        })) as Box<dyn geng::State>
    });
}
