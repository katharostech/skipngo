dev_features:='--features bevy/dynamic'

build:
    cargo build {{dev_features}}

build-web game='demo1':
    cargo build --target wasm32-unknown-unknown
    wasm-bindgen --out-dir target/wasm --target web target/wasm32-unknown-unknown/debug/skipngo.wasm
    cp wasm_resources/index.tpl.html target/wasm/index.html
    rm -rf target/wasm/assets
    mkdir -p target/wasm
    ln -fs ../../games/{{game}} target/wasm/assets

run *args:
    cargo run {{dev_features}} -- {{args}}

run-web game='demo1': build-web
    @echo "Debug link: http://localhost:4000?RUST_LOG=debug"
    basic-http-server -x target/wasm
