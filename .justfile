run *OPTIONS:
    cargo run --features bevy/dynamic_linking {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}

publish:
    trunk build --release
    butler push dist kuviman/bevy-jam-3:html5

clean:
    cargo clean

prepare *OPTIONS:
    cargo check {{OPTIONS}}
    cargo check --target wasm32-unknown-unknown {{OPTIONS}}
    cargo build --features bevy/dynamic_linking {{OPTIONS}}
    cargo build --target wasm32-unknown-unknown {{OPTIONS}}
