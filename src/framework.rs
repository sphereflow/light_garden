use crate::gui::UiMode;
use crate::light_garden::LightGarden;
use crate::{gui::Gui, renderer::Renderer};
use std::sync::Arc;
use wgpu::Adapter;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};

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
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter, // what is the difference btw Adapter and Device ?
    instance: wgpu::Instance,
    renderer: Renderer,
    egui_state: egui_winit::State,
}

pub async fn setup(window: Arc<winit::window::Window>, app: &mut LightGarden) -> Setup {
    #[cfg(target_arch = "wasm32")]
    {
        let mut width = width;
        let mut height = height;
        use winit::platform::web::WindowExtWebSys;
        console_log::init().expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                width = body.client_width() as u32;
                height = body.client_height() as u32;
                window.set_inner_size(winit::dpi::PhysicalSize::new(width, height));
                window
                    .canvas()
                    .set_oncontextmenu(Some(&js_sys::Function::new_no_args("return false;")));
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    log::info!("Initializing the surface...");

    // wgpu instance creates adapters and surfaces
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
    {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    #[cfg(not(target_arch = "wasm32"))]
    let needed_limits = wgpu::Limits::default().using_resolution(adapter.limits());

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
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        desired_maximum_frame_latency: 2,
        view_formats: Vec::new(),
    };
    println!("Surface config: {surface_config:?}");
    surface.configure(&device, &surface_config);

    let renderer = Renderer::init(&surface_config, &device, &queue, app);

    let egui_state = egui_winit::State::new(
        egui::Context::default(),
        egui::viewport::ViewportId::ROOT,
        &window.clone(),
        Some(window.scale_factor() as f32),
        None,
        Some(2048),
    );

    Setup {
        window,
        instance,
        size,
        surface,
        surface_config,
        adapter,
        device,
        queue,
        renderer,
        egui_state,
    }
}

pub struct AppState {
    gui: Gui,
    setup: Option<Setup>,
}

impl AppState {
    fn new(app: LightGarden) -> Self {
        let gui = Gui::new(app);
        AppState { gui, setup: None }
    }
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(winit::window::Window::default_attributes())
                .expect("Gui::resumed() could not create window"),
        );
        let setup = pollster::block_on(crate::framework::setup(window.clone(), &mut self.gui.app));
        self.gui.app.resumed(&setup.surface_config);
        self.setup = Some(setup);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        log::info!("Entering render loop...");
        if let AppState {
            gui,
            setup: Some(setup),
        } = self
        {
            let Setup {
                window,
                device,
                queue,
                size: _,
                surface,
                surface_config,
                adapter: _,
                instance: _,
                renderer,
                egui_state,
            } = setup;
            gui.winit_update(&event, surface_config);
            match event {
                WindowEvent::RedrawRequested => {
                    let surface_texture = match surface.get_current_texture() {
                        Ok(frame) => frame,
                        Err(_) => {
                            surface.configure(device, surface_config);
                            surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture!")
                        }
                    };
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
                    if let Some(path) = gui.app.screenshot_path.take() {
                        #[cfg(not(target_arch = "wasm32"))]
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
                    println!("Resized: {size:?}");
                    surface_config.width = size.width.max(1);
                    surface_config.height = size.height.max(1);
                    renderer.resize(surface_config, device, queue, &mut gui.app);
                    surface.configure(device, surface_config);
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

#[cfg(not(target_arch = "wasm32"))]
pub fn wgpu_main() {
    use collision2d::geo::Rect;

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    };

    let event_loop = EventLoop::new().expect("could not create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let app = LightGarden::new(Rect::from_tlbr(1.0, -1.0, -1.0, 1.0));
    let mut app_state = AppState::new(app);
    event_loop.run_app(&mut app_state).unwrap();
}

#[cfg(target_arch = "wasm32")]
pub fn wgpu_main(width: u32, height: u32) {
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup("LightGarden", width, height).await;
        start(setup);
    });
}
