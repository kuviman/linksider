use geng::prelude::*;

use std::ops::ControlFlow;

mod buttons;
mod config;
mod editor;
mod history;
mod input;
mod level_select;
mod levels;
mod play;
mod popup;
mod renderer;
mod sound;
mod util;

use buttons::{Anchor, Button};
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

#[cfg(target_os = "android")]
mod android_main {
    use super::*;

    #[no_mangle]
    fn android_main(app: android::App) {
        android::init(app);
        android::set_file_mode(android::FileMode::FileSystem);
        if !run_dir().join("levels").exists() {
            android::copy_assets_to_filesystem(["levels"], run_dir());
        }
        if run_dir().join("assets").exists() {
            std::fs::remove_dir_all(run_dir().join("assets")).unwrap();
        }
        android::copy_assets_to_filesystem(["assets"], run_dir());
        super::main();
    }
}

pub fn main() {
    logger::init();
    geng::setup_panic_handler();
    let cli_args: Opt = cli::parse();
    Geng::run_with(
        &{
            let mut options = geng::ContextOptions {
                window: geng::window::Options::new("LinkSider"),
                ..default()
            };
            options.with_cli(&cli_args.geng);
            options
        },
        move |geng| async move {
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

            if cli_args.editor {
                editor::world::State::load(&ctx).await;
            } else {
                level_select::run(&ctx).await;
            }
        },
    );
}
