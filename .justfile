run:
    cargo run --features bevy/dynamic_linking

playground:
    cargo run --features bevy/dynamic_linking --bin playground

web:
    trunk serve --open

publish:
    trunk build --release playground.html
    butler push dist kuviman/bevy-jam-3:html5
