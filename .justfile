run *OPTIONS:
    cargo run {{OPTIONS}}

web *OPTIONS:
    trunk serve --open {{OPTIONS}}

publish-web *OPTIONS:
    trunk --config Trunk.itch-io.toml build {{OPTIONS}}
    butler push dist kuviman/linksider:html5

publish-native *OPTIONS:
    #!/usr/bin/env sh
    folder=debug
    for opt in {{OPTIONS}}; do
        if [[ $opt = "--release" ]]; then
            folder=release
        fi
    done
    cargo build {{OPTIONS}}
    rm -rf dist || true
    mkdir dist
    cp target/$folder/linksider.exe dist/
    cp -r assets dist/
    butler push dist kuviman/linksider:windows

publish *OPTIONS:
    just publish-web {{OPTIONS}}
    just publish-native {{OPTIONS}}

clean:
    cargo clean

prepare *OPTIONS:
    cargo check {{OPTIONS}}
    cargo check --target wasm32-unknown-unknown {{OPTIONS}}
    cargo build {{OPTIONS}}
    cargo build --target wasm32-unknown-unknown {{OPTIONS}}
