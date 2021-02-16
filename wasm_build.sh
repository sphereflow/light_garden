#!/bin/bash
if [ ! -d "target" ]
then
  echo "Directory target not found! Make sure you are in the project root."
  echo "exiting!"
  exit
fi
if [ ! -d "target/generated" ]
then
  if [ ! -d "wasm_resources" ]
  then
    echo "wasm_resources directory not found ... exiting"
    exit
  fi
  mkdir "target/generated"
  cp ./wasm_resources/* ./target/generated
fi
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --release --target wasm32-unknown-unknown
echo "wasm-bindgen"
wasm-bindgen --out-dir target/generated --web target/wasm32-unknown-unknown/release/light_garden.wasm
