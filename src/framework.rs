use crate::gui::UiMode;
use crate::light_garden::LightGarden;
use crate::{gui::Gui, renderer::Renderer};
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use wgpu::Adapter;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

#[rustfmt::skip]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::{mem::size_of_val, slice::from_raw_parts};
    unsafe { from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) }
}

pub struct Setup {
    window: Arc<winit::window::Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    renderer: Renderer,
    egui_state: egui_winit::State,
    gui: Gui,
}

pub async fn setup(window: Arc<winit::window::Window>, proxy: EventLoopProxy<Setup>) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    // create the main rendering surface (on screen or window)
    let (size, surface) = {
        let size = window.inner_size();
        let surface = instance
            .create_surface(window.clone())
            .expect("wgpu::Instance::create_surface failed");
        (size, surface)
    };

    let adapter: Adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    let optional_features = wgpu::Features::empty();
    let required_features = wgpu::Features::empty();
    let adapter_features = adapter.features();
    assert!(
        adapter_features.contains(required_features),
        "Adapter does not support required features for this example: {:?}",
        required_features - adapter_features
    );
    println!("Features: {adapter_features:?}");

    #[cfg(not(target_arch = "wasm32"))]
    let needed_limits = {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
        wgpu::Limits::default().using_resolution(adapter.limits())
    };

    #[cfg(target_arch = "wasm32")]
    let needed_limits =
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Framework: device descriptor"),
            required_features: (optional_features & adapter_features) | required_features,
            required_limits: needed_limits,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Cannot request GPU device");

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_capabilities(&adapter).formats[0],
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        desired_maximum_frame_latency: 2,
        view_formats: Vec::new(),
    };

    let mut app = LightGarden::new(
        collision2d::geo::Rect::from_tlbr(1.0, -1.0, -1.0, 1.0),
        &surface_config,
    );
    let renderer = Renderer::init(&surface_config, &device, &queue, &mut app);

    let egui_state = egui_winit::State::new(
        egui::Context::default(),
        egui::viewport::ViewportId::ROOT,
        &window.clone(),
        Some(window.scale_factor() as f32),
        None,
        Some(2048),
    );

    let gui = Gui::new(app);

    let _ = proxy.send_event(Setup {
        window,
        surface,
        surface_config,
        is_surface_configured: false,
        device,
        queue,
        renderer,
        egui_state,
        gui,
    });
}

pub enum AppState {
    Init(Option<EventLoopProxy<Setup>>),
    Done(Setup),
}

impl ApplicationHandler<Setup> for AppState {
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, setup: Setup) {
        if let AppState::Init(_) = self {
            setup.window.request_redraw();
            *self = AppState::Done(setup);
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        use winit::window::Window;
        if let AppState::Init(proxy) = self {
            if let Some(proxy) = proxy.take() {
                #[allow(unused_mut)]
                let mut window_attributes = Window::default_attributes();
                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::JsCast;
                    use web_sys::{Document, Element, HtmlCanvasElement};
                    use winit::platform::web::WindowAttributesExtWebSys;

                    const CANVAS_ID: &str = "canvas";

                    let window: wgpu::web_sys::Window =
                        wgpu::web_sys::window().expect("web_sys::window()");
                    let document: Document = window.document().expect("window.document()");
                    let canvas: Element = document
                        .get_element_by_id(CANVAS_ID)
                        .expect("document.get_element_by_id()");
                    let html_canvas_element: HtmlCanvasElement = canvas.unchecked_into();
                    let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
                    let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
                    html_canvas_element.set_width(width);
                    html_canvas_element.set_height(height);
                    window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
                }
                let window = Arc::new(
                    event_loop
                        .create_window(window_attributes)
                        .expect("AppState::resumed() could not create window"),
                );
                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(async move { setup(window, proxy).await });
                #[cfg(not(target_arch = "wasm32"))]
                pollster::block_on(setup(window, proxy));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let AppState::Done(setup) = self {
            let Setup {
                window,
                device,
                queue,
                surface,
                surface_config,
                is_surface_configured,
                renderer,
                egui_state,
                gui,
            } = setup;
            gui.winit_update(&event, surface_config);
            match event {
                WindowEvent::RedrawRequested => {
                    if !*is_surface_configured {
                        surface.configure(device, surface_config);
                        *is_surface_configured = true;
                    }
                    let surface_texture = surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture!");
                    let raw_input = egui_state.take_egui_input(window);
                    egui_state.egui_ctx().begin_pass(raw_input);
                    gui.update(egui_state.egui_ctx());

                    let output = egui_state.egui_ctx().end_pass();

                    renderer.render(
                        &surface_texture,
                        device,
                        queue,
                        &mut gui.app,
                        output,
                        egui_state.egui_ctx(),
                        window.scale_factor() as f32,
                    );
                    surface_texture.present();
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(path) = gui.app.screenshot_path.take() {
                        pollster::block_on(renderer.make_screenshot(
                            path,
                            device,
                            queue,
                            gui.app.get_render_to_texture(),
                        ));
                    }
                    setup.window.request_redraw();
                }
                WindowEvent::Resized(size) => {
                    log::info!("Resizing to {size:?}");
                    surface_config.width = size.width.max(1);
                    surface_config.height = size.height.max(1);
                    renderer.resize(surface_config, device, queue, &mut gui.app);
                    surface.configure(device, surface_config);
                    *is_surface_configured = true;
                }
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                _ => {
                    // forward events to egui
                    let _ = egui_state.on_window_event(window, &event);
                }
            }

            if gui.ui_mode == UiMode::Exiting {
                event_loop.exit();
            }

            gui.app.update();
        }
    }
}

pub fn wgpu_main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    };

    let event_loop = EventLoop::<Setup>::with_user_event()
        .build()
        .expect("could not create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    #[cfg_attr(target_arch = "wasm32", expect(unused_mut))]
    let mut app_state = AppState::Init(Some(event_loop.create_proxy()));
    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run_app(&mut app_state).unwrap();
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("Could not init logger");
        use winit::platform::web::EventLoopExtWebSys;
        wasm_bindgen_futures::spawn_local(async move {
            event_loop.spawn_app(app_state);
        });
    }
}
