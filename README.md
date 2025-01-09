# intro-rs

A framework for writing 4k intros in Rust.

This is a fork of [sphere_dance](https://github.com/janiorca/sphere_dance). A more detailed explanation about the different size optimizations can be found [here](https://www.codeslow.com/2020/07/writing-winning-4k-intro-in-rust.html)

## Requirements

* Windows 10 SDK (10.0.18362.0): Install from Visual Studio Installer - https://visualstudio.microsoft.com/downloads
* Crinkler: `git submodule update --init`
* cargo: https://www.rust-lang.org/tools/install
* xargo: `cargo install xargo`
* rustup: `rustup install nightly-2023-05-01 && rustup default nightly-2023-05-01`
* rust-src: `rustup component add rust-src`

## Commands

* Build: `./scripts/build.sh`
* Package: `./scripts/package.sh`
* Run: `./scripts/run.sh`
* Shader minify: `./tools/shader_minifier.exe ./shader.glsl --preserve-externals --format none`
