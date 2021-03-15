dev_features:='--features bevy/dynamic'

build:
    cargo build {{dev_features}}

build-web:
    cargo build --target wasm32-unknown-unknown
    wasm-bindgen --out-dir target/wasm --target web target/wasm32-unknown-unknown/debug/skipngo.wasm
    cp wasm_resources/index.tpl.html target/wasm/index.html
    ln -fs ../../../assets target/wasm

run:
    cargo run {{dev_features}}

run-web: build-web
    basic-http-server target/wasm
