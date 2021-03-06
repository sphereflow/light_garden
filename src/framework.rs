use crate::gui::{Gui, UiMode};
use crate::renderer::Renderer;
#[cfg(not(target_arch = "wasm32"))]
use futures_lite::future;
use winit::{
    dpi::LogicalSize,
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

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
    use std::{mem::size_of, slice::from_raw_parts};

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

struct Setup {
    window: winit::window::Window,
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter, // what is the difference btw Adapter and Device ?
    device: wgpu::Device,
    queue: wgpu::Queue,
}

async fn setup(title: &str, width: u32, height: u32) -> Setup {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let chrome_tracing_dir = std::env::var("WGPU_CHROME_TRACE");
        wgpu_subscriber::initialize_default_subscriber(
            chrome_tracing_dir.as_ref().map(std::path::Path::new).ok(),
        );
    };

    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new()
        .with_inner_size(LogicalSize::new(width as f32, height as f32));
    builder = builder.with_title(title);
    #[cfg(windows_OFF)] // TODO
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        console_log::init().expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    log::info!("Initializing the surface...");

    let instance = wgpu::Instance::new(wgpu::BackendBit::all());
    let (size, surface) = unsafe {
        let size = window.inner_size();
        let surface = instance.create_surface(&window);
        (size, surface)
    };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("No suitable GPU found");

    let optional_features = wgpu::Features::empty();
    let required_features = wgpu::Features::empty();
    let adapter_features = adapter.features();
    assert!(
        adapter_features.contains(required_features),
        "Adapter does not support required features for this example: {:?}",
        required_features - adapter_features
    );

    let needed_limits = wgpu::Limits::default();

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & adapter_features) | required_features,
                limits: needed_limits,
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .expect("Cannot request GPU device");

    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }
}

fn start(
    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }: Setup,
) {
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: adapter.get_swap_chain_preferred_format(&surface),
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    log::info!("Initializing the example...");
    let mut gui = Gui::new(&window, &sc_desc);
    let mut renderer = Renderer::init(&sc_desc, &device, &adapter, &queue, &mut gui.app);
    log::info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter); // force ownership by the closure
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use instant::{Instant, Duration};
                ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(10))
            }
            #[cfg(target_arch = "wasm32")]
            {
                ControlFlow::Poll
            }
        };

        match event {
            event::Event::MainEventsCleared => {
                window.request_redraw();
            }
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                log::info!("Resizing to {:?}", size);
                sc_desc.width = if size.width == 0 { 1 } else { size.width };
                sc_desc.height = if size.height == 0 { 1 } else { size.height };
                renderer.resize(&sc_desc, &device, &queue, &mut gui.app);
                swap_chain = device.create_swap_chain(&surface, &sc_desc);
            }
            event::Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {
                    gui.update(event, &sc_desc);
                }
            },
            event::Event::RedrawRequested(_) => {
                let frame = match swap_chain.get_current_frame() {
                    Ok(frame) => frame,
                    Err(_) => {
                        swap_chain = device.create_swap_chain(&surface, &sc_desc);
                        swap_chain
                            .get_current_frame()
                            .expect("Failed to acquire next swap chain texture!")
                    }
                };
                renderer.render(&frame.output, &device, &queue, &mut gui);
            }

            _ => {}
        }

        if gui.ui_mode == UiMode::Exiting {
            *control_flow = ControlFlow::Exit;
        }
        gui.platform.handle_event(&event);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn wgpu_main(width: u32, height: u32) {
    let setup = future::block_on(setup("LightGarden", width, height));
    start(setup);
}

#[cfg(target_arch = "wasm32")]
pub fn wgpu_main(width: u32, height: u32) {
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup("LightGarden", width, height).await;
        start(setup);
    });
}
