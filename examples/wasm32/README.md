# Intro 

This is a demo to show how to use crossbus in WebAssembly

Alternatively, apply `crossbus` to other similar wasm template
just follow similar pattern

# How to Use 

this demo is a simple itegration with [yew](https://yew.rs).
To preview this, you should install 
- Rust, of course

- [Trunk](https://trunkrs.dev/)
  `cargo install trunk`

- WebAssembly toolchain and compilation target
  `rustup target add wasm32-unknown-unknown`

After these, go to the root directory of the project, run 
```
trunk serve
```
then open your browser and go to the `localhost:8080` to preview

