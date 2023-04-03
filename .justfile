run *OPTIONS:
    cargo run --features bevy/dynamic_linking {{OPTIONS}}

playground *OPTIONS:
    cargo run --features bevy/dynamic_linking --bin playground {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}

publish:
    trunk build --release playground.html
    butler push dist kuviman/bevy-jam-3:html5
