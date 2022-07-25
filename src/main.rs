extern crate nalgebra as na;

mod framework;
mod gui;
pub mod light_garden;
pub mod renderer;
mod sub_render_pass;
mod texture_renderer;

const WIDTH: u32 = 1800;
const HEIGHT: u32 = 1000;

pub fn main() {
    framework::wgpu_main(WIDTH, HEIGHT);
}
