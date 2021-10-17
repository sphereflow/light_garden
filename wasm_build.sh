#!/bin/bash
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --release --target wasm32-unknown-unknown
if [ ! -d "target" ]
then
    echo "Directory target not found! Make sure you are in the project root."
    echo "exiting!"
    exit
fi
if [ ! -d "target/generated" ]
then
    mkdir "target/generated"
fi
if [ ! -d "wasm_resources" ]
then
    echo "wasm_resources directory not found ... exiting"
exit
fi
cp ./wasm_resources/* ./target/generated
echo "wasm-bindgen"
wasm-bindgen --target web --out-dir target/generated target/wasm32-unknown-unknown/release/light_garden.wasm
