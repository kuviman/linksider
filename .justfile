run *OPTIONS:
    cargo run --features bevy/dynamic_linking {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}

publish:
    trunk build --release
    butler push dist kuviman/bevy-jam-3:html5
