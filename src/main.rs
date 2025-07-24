extern crate nalgebra as na;

mod framework;
mod gui;
pub mod light_garden;
pub mod renderer;
mod sub_render_pass;
mod texture_renderer;

pub fn main() {
    framework::wgpu_main();
}
