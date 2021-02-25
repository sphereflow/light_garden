extern crate nalgebra as na;

pub mod egui_renderer;
mod framework;
mod gui;
pub mod light_garden;
pub mod renderer;
mod texture_renderer;

const WIDTH: u32 = 1800;
const HEIGHT: u32 = 1000;

pub fn main() {
    framework::wgpu_main(WIDTH, HEIGHT);
}
