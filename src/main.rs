extern crate nalgebra as na;

mod gui;
mod framework;
pub mod light_garden;
pub mod renderer;

const WIDTH: u32 = 1800;
const HEIGHT: u32 = 1000;

pub fn main() {
    framework::wgpu_main(WIDTH, HEIGHT);
}
