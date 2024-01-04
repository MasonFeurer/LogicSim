set -e
# may need to first run `cargo install wasm-bindgen-cli`
RUSTFLAGS='--cfg=web_sys_unstable_apis' cargo build --profile web_release --target wasm32-unknown-unknown
wasm-bindgen --out-dir site --no-modules --no-typescript ../target/wasm32-unknown-unknown/web_release/logisim_web.wasm
