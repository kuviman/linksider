run *OPTIONS:
    cargo run --features bevy/dynamic_linking {{OPTIONS}}

playground *OPTIONS:
    cargo run --features bevy/dynamic_linking --bin playground {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}