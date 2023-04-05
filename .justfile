run *OPTIONS:
    cargo run {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}

publish:
    trunk --config Trunk.itch-io.toml build --release
    butler push dist kuviman/bevy-jam-3:html5

clean:
    cargo clean

prepare *OPTIONS:
    cargo check {{OPTIONS}}
    cargo check --target wasm32-unknown-unknown {{OPTIONS}}
    cargo build {{OPTIONS}}
    cargo build --target wasm32-unknown-unknown {{OPTIONS}}
