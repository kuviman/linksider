run *OPTIONS:
    cargo run {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}

publish-web *OPTIONS:
    trunk --config Trunk.itch-io.toml build {{OPTIONS}}
    butler push dist kuviman/linksider:html5

publish:
    cargo build
    rm -rf dist || true
    mkdir dist
    cp target/debug/linksider.exe dist/
    cp -r assets dist/
    butler push dist kuviman/linksider:windows

clean:
    cargo clean

prepare *OPTIONS:
    cargo check {{OPTIONS}}
    cargo check --target wasm32-unknown-unknown {{OPTIONS}}
    cargo build {{OPTIONS}}
    cargo build --target wasm32-unknown-unknown {{OPTIONS}}
