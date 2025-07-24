use winit::keyboard::NamedKey;

use super::*;

impl Gui {
    pub fn winit_update(
        &mut self,
        event: &winit::event::WindowEvent,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        use winit::event;
        use winit::event::WindowEvent;
        use winit::keyboard::Key;

        match event {
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if winit::event::ElementState::Released == event.state {
                    match (event.logical_key.as_ref(), self.ui_mode) {
                        (Key::Named(NamedKey::Escape), ui_mode) => match ui_mode {
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
                                self.app.mode = Mode::StringMod;
                                self.ui_mode = UiMode::Main;
                            }
                            UiMode::TileMap => {
                                self.ui_mode = UiMode::Main;
                                self.app.mode = Mode::Selecting(None);
                            }
                            UiMode::Exiting => {}
                        },
                        (Key::Character("b"), _ui_mode) => self.app.tracer.debug_key_pressed = true,
                        (Key::Character("a"), UiMode::Main) => self.ui_mode = UiMode::Add,
                        (Key::Character("e"), UiMode::Main) => self.ui_mode = UiMode::Settings,
                        (Key::Character("t"), UiMode::Main) => {
                            self.ui_mode = UiMode::TileMap;
                            self.app.mode = Mode::SelectTile;
                        }
                        (Key::Character("r"), UiMode::Main) => {
                            self.app.mode = Mode::StringMod;
                            self.ui_mode = UiMode::StringMod;
                        }

                        (Key::Character("p"), UiMode::Add) => self.app.mode = Mode::DrawPointLight,
                        (Key::Character("s"), UiMode::Add) => {
                            self.app.mode = Mode::DrawSpotLightStart
                        }
                        (Key::Character("d"), UiMode::Add) => {
                            self.app.mode = Mode::DrawDirectionalLightStart
                        }

                        (Key::Character("r"), UiMode::Add) => self.app.mode = Mode::DrawRectStart,
                        (Key::Character("c"), UiMode::Add) => self.app.mode = Mode::DrawCircleStart,
                        (Key::Character("m"), UiMode::Add) => self.app.mode = Mode::DrawMirrorStart,
                        (Key::Character("v"), UiMode::Add) => {
                            self.app.mode = Mode::DrawConvexPolygon { points: Vec::new() }
                        }
                        (Key::Character("u"), UiMode::Add) => {
                            self.app.mode = Mode::DrawCurvedMirror { points: Vec::new() }
                        }

                        (Key::Character("e"), UiMode::Selected) => self.app.mode = Mode::EditObject,
                        (Key::Character("r"), UiMode::Selected) => self.app.mode = Mode::Rotate,
                        (Key::Character("a"), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::And))
                        }
                        (Key::Character("o"), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::Or))
                        }
                        (Key::Character("n"), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::AndNot))
                        }
                        (Key::Character("d"), UiMode::Selected) => {
                            self.app.delete_selected();
                            self.ui_mode = UiMode::Main;
                        }
                        (Key::Character("c"), UiMode::Selected) => self.app.copy_selected(),
                        (Key::Character("x"), UiMode::Selected) => {
                            self.app.mirror_on_x_axis_selected()
                        }
                        (Key::Character("y"), UiMode::Selected) => {
                            self.app.mirror_on_y_axis_selected()
                        }

                        (Key::Character("q"), _) => self.ui_mode = UiMode::Exiting,

                        _ => {}
                    }
                }
                if let (Key::Named(NamedKey::Shift), winit::event::ElementState::Pressed) =
                    (event.logical_key.as_ref(), event.state)
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
