run:
  cargo run

test:
  cargo test --workspace

android:
  rm -rf android-assets || true
  mkdir android-assets
  cp -r assets levels android-assets/
  CARGO_APK_RELEASE_KEYSTORE=$HOME/.android/debug.keystore CARGO_APK_RELEASE_KEYSTORE_PASSWORD=android cargo apk run --release
