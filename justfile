dev_features:='--features bevy/dynamic'

build:
    cargo build {{dev_features}}

build-release:
    cargo build --release
    strip target/release/skipngo

build-cross-windows:
    cargo build --target x86_64-pc-windows-gnu

build-release-cross-windows:
    cargo build --release --target x86_64-pc-windows-gnu
    strip target/x86_64-pc-windows-gnu/release/skipngo.exe

build-web:
    cargo build --target wasm32-unknown-unknown
    wasm-bindgen --out-dir target/wasm --target web target/wasm32-unknown-unknown/debug/skipngo.wasm
    cp wasm_resources/index.tpl.html target/wasm/index.html
    mkdir -p target/wasm

build-release-web:
    cargo build --target wasm32-unknown-unknown --release
    wasm-bindgen --out-dir target/wasm-dist --no-typescript --target web target/wasm32-unknown-unknown/release/skipngo.wasm
    cp wasm_resources/index.tpl.html target/wasm-dist/index.html

run *args:
    cargo run {{dev_features}} -- {{args}}

run-web: build-web
    @echo "Debug link: http://localhost:4000?RUST_LOG=debug"
    basic-http-server -x target/wasm
