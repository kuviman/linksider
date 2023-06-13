run:
  cargo run

test:
  cargo test --workspace

android:
  rm -rf android-assets || true
  mkdir android-assets
  cp -r assets levels android-assets/
  cargo apk run
