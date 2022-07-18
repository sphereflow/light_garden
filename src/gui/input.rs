use super::*;

impl Gui {
    pub fn winit_update(
        &mut self,
        event: &winit::event::WindowEvent,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        use winit::event;
        use winit::event::WindowEvent;
        type Key = event::VirtualKeyCode;
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. } => {
                if winit::event::ElementState::Released == input.state {
                    match (input.virtual_keycode, self.ui_mode) {
                        (Some(Key::Escape), ui_mode) => match ui_mode {
                            UiMode::Main => {}
                            UiMode::Add => {
                                self.ui_mode = UiMode::Main;
                                self.app.mode = Mode::Selecting(None);
                            }
                            UiMode::Selected => {
                                self.app.deselect();
                                self.ui_mode = UiMode::Main;
                            }
                            UiMode::Settings => self.ui_mode = UiMode::Main,
                            UiMode::Grid => self.ui_mode = UiMode::Main,
                            UiMode::StringMod => {
                                self.app.mode = Mode::NoMode;
                                self.ui_mode = UiMode::Main;
                            }
                            UiMode::TileMap => {
                                self.ui_mode = UiMode::Main;
                                self.app.mode = Mode::Selecting(None);
                            }
                            UiMode::Exiting => {}
                        },
                        (Some(Key::B), _ui_mode) => self.app.tracer.debug_key_pressed = true,
                        (Some(Key::A), UiMode::Main) => self.ui_mode = UiMode::Add,
                        (Some(Key::E), UiMode::Main) => self.ui_mode = UiMode::Settings,
                        (Some(Key::T), UiMode::Main) => {
                            self.ui_mode = UiMode::TileMap;
                            self.app.mode = Mode::SelectTile;
                        }
                        (Some(Key::R), UiMode::Main) => {
                            self.app.mode = Mode::StringMod;
                            self.ui_mode = UiMode::StringMod;
                        }

                        (Some(Key::P), UiMode::Add) => self.app.mode = Mode::DrawPointLight,
                        (Some(Key::S), UiMode::Add) => self.app.mode = Mode::DrawSpotLightStart,
                        (Some(Key::D), UiMode::Add) => {
                            self.app.mode = Mode::DrawDirectionalLightStart
                        }

                        (Some(Key::R), UiMode::Add) => self.app.mode = Mode::DrawRectStart,
                        (Some(Key::C), UiMode::Add) => self.app.mode = Mode::DrawCircleStart,
                        (Some(Key::M), UiMode::Add) => self.app.mode = Mode::DrawMirrorStart,
                        (Some(Key::V), UiMode::Add) => {
                            self.app.mode = Mode::DrawConvexPolygon { points: Vec::new() }
                        }
                        (Some(Key::U), UiMode::Add) => {
                            self.app.mode = Mode::DrawCurvedMirror { points: Vec::new() }
                        }

                        (Some(Key::E), UiMode::Selected) => self.app.mode = Mode::EditObject,
                        (Some(Key::R), UiMode::Selected) => self.app.mode = Mode::Rotate,
                        (Some(Key::A), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::And))
                        }
                        (Some(Key::O), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::Or))
                        }
                        (Some(Key::N), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::AndNot))
                        }
                        (Some(Key::D), UiMode::Selected) => {
                            self.app.delete_selected();
                            self.ui_mode = UiMode::Main;
                        }
                        (Some(Key::C), UiMode::Selected) => self.app.copy_selected(),
                        (Some(Key::X), UiMode::Selected) => self.app.mirror_on_x_axis_selected(),
                        (Some(Key::Y), UiMode::Selected) => self.app.mirror_on_y_axis_selected(),

                        (Some(Key::Q), _) => self.ui_mode = UiMode::Exiting,

                        _ => {}
                    }
                }
                if let (Some(Key::LShift), winit::event::ElementState::Pressed) =
                    (input.virtual_keycode, input.state)
                {
                    self.app.tracer.grid.on = true;
                } else {
                    self.app.tracer.grid.on = false;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let aspect = surface_config.width as f64 / surface_config.height as f64;
                self.app.update_mouse_position(nalgebra::Point2::new(
                    ((2. * position.x / (surface_config.width as f64)) - 1.) * aspect,
                    (2. * -position.y / (surface_config.height as f64)) + 1.,
                ));
            }
            WindowEvent::MouseInput {
                state: event::ElementState::Pressed,
                button: event::MouseButton::Left,
                ..
            } => {
                if !self.gui_contains_pointer {
                    self.app.mouse_down();
                }
            }
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Left,
                ..
            } => {
                if !self.gui_contains_pointer {
                    self.app.mouse_released();
                }
            }
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Right,
                ..
            } => {
                match (self.ui_mode, &self.app.mode) {
                    (UiMode::TileMap, Mode::TileSelected { .. }) => {
                        self.app.mode = Mode::SelectTile
                    }
                    (UiMode::TileMap, Mode::SelectTile) => {
                        self.ui_mode = UiMode::Main;
                        self.app.mode = Mode::Selecting(None);
                    }
                    (_, _) => {
                        self.ui_mode = UiMode::Main;
                        self.app.mode = Mode::Selecting(None)
                    }
                }
                self.app.deselect();
            }
            _ => {}
        }
    }
}
