//! The parts of this example enabling MSAA are:
//! *    The render pipeline is created with a sample_count > 1.
//! *    A new texture with a sample_count > 1 is created and set as the color_attachment instead of the swapchain.
//! *    The swapchain is now specified as a resolve_target.
//!
//! The parts of this example enabling LineList are:
//! *   Set the primitive_topology to PrimitiveTopology::LineList.
//! *   Vertices and Indices describe the two points that make up a line.

use std::iter;

use crate::egui_renderer::EguiRenderer;
use crate::gui::Gui;
use crate::light_garden::light::Color;
use crate::light_garden::LightGarden;
use bytemuck::{Pod, Zeroable};
use collision2d::geo::*;
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    _pos: [f32; 2],
    _color: [f32; 4],
}
unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

pub struct Renderer {
    bundle: Option<RenderBundle>,
    shader: ShaderModule,
    pipeline: RenderPipeline,
    multisampled_framebuffer: TextureView,
    vertex_buffer: Buffer,
    vertex_count: u32,
    sample_count: u32,
    matrix_bind_group: BindGroup,
    rebuild_bundle: bool,
    sc_desc: SwapChainDescriptor,
    egui_renderer: EguiRenderer,
}

impl Renderer {
    // this function is called by Example::init(...) and Example::render(...)
    // encoder.finish(...) creates a RenderBundle
    fn create_bundle(&mut self, device: &Device, queue: &Queue, app: &mut LightGarden) {
        log::info!("sample_count: {}", self.sample_count);
        let (pipeline, bind_group) =
            Renderer::create_pipeline(&self.sc_desc, device, queue, &self.shader, app);
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
        let mut encoder = device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
            label: None,
            color_formats: &[self.sc_desc.format],
            depth_stencil_format: None,
            sample_count: self.sample_count,
        });
        encoder.set_pipeline(&self.pipeline);
        encoder.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // slot 0
        encoder.draw(0..self.vertex_count, 0..1); // vertex range, instance range
        self.bundle = Some(encoder.finish(&RenderBundleDescriptor {
            label: Some("primitives render bundle"),
        }));
    }

    fn create_pipeline(
        sc_desc: &SwapChainDescriptor,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        app: &mut LightGarden,
    ) -> (RenderPipeline, BindGroup) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });

        let aspect = sc_desc.width as f32 / sc_desc.height as f32;
        let mx = Self::generate_matrix(aspect);
        let mx_ref: &[f32; 16] = mx.as_ref();
        let mx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("u_Transform"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        app.canvas_bounds = Rect::from_tlbr(1., -aspect as f64, -1., aspect as f64);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("u_Transform"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer {
                    buffer: &mx_buf,
                    offset: 0,
                    size: None,
                },
            }],
        });
        queue.write_buffer(&mx_buf, 0, bytemuck::cast_slice(mx_ref));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        (
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("render pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float4],
                    }],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[app.color_state_descriptor.clone()],
                }),
                // render lines
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::LineList,
                    front_face: FrontFace::Ccw,
                    ..Default::default()
                },
                depth_stencil: None,
                // vertex_state: VertexStateDescriptor {
                // index_format: IndexFormat::Uint16,
                // vertex_buffers: &[VertexBufferDescriptor {
                // stride: std::mem::size_of::<Vertex>() as BufferAddress,
                // step_mode: InputStepMode::Vertex,
                // attributes: &vertex_attr_array![0 => Float2, 1 => Float4],
                // }],
                // },
                multisample: MultisampleState {
                    ..Default::default()
                },
            }),
            bind_group,
        )
    }

    fn create_multisampled_framebuffer(
        device: &Device,
        sc_desc: &SwapChainDescriptor,
        sample_count: u32,
    ) -> TextureView {
        let multisampled_texture_extent = Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        };
        let multisampled_frame_descriptor = &TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: sc_desc.format,
            usage: TextureUsage::RENDER_ATTACHMENT,
            label: None,
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&TextureViewDescriptor::default())
    }

    pub fn update_vertex_buffer(&mut self, device: &Device, vertices: &Vec<(P2, Color)>) {
        let vertex_data: Vec<Vertex> = vertices
            .iter()
            .map(|(p, color)| Vertex {
                _pos: [p.x as f32, p.y as f32],
                _color: *color,
            })
            .collect();
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: BufferUsage::VERTEX,
        });
        self.vertex_count = vertex_data.len() as u32;
        self.rebuild_bundle = true;
    }

    pub fn update_vertex_buffer_with_line_strips(
        &mut self,
        device: &Device,
        vertices: &Vec<(Vec<P2>, Color)>,
    ) {
        let mut vertex_data: Vec<Vertex> = Vec::with_capacity(vertices.len() * 2);
        for (line_strip, color) in vertices {
            for w in line_strip.windows(2) {
                vertex_data.push(Vertex {
                    _pos: [w[0].x as f32, w[0].y as f32],
                    _color: *color,
                });
                vertex_data.push(Vertex {
                    _pos: [w[1].x as f32, w[1].y as f32],
                    _color: *color,
                });
            }
        }
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: BufferUsage::VERTEX,
        });
        self.vertex_count = vertex_data.len() as u32;
        self.rebuild_bundle = true;
    }

    pub fn init(
        sc_desc: &SwapChainDescriptor,
        device: &Device,
        adapter: &Adapter,
        queue: &Queue, // we might need to meddle with the command queue
        app: &mut LightGarden,
    ) -> Self {
        log::info!("Press left/right arrow keys to change sample_count.");
        let sample_count = 1;

        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match adapter.get_info().backend {
            wgpu::Backend::Metal | wgpu::Backend::Vulkan => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
            }
            _ => (), //TODO
        }

        use std::borrow::Cow;
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
            flags,
        });

        let multisampled_framebuffer: TextureView =
            Renderer::create_multisampled_framebuffer(device, sc_desc, sample_count);

        // create the vertex buffer
        let vertex_data: Vec<Vertex> = vec![];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: BufferUsage::VERTEX,
        });
        let vertex_count = vertex_data.len() as u32;
        let (pipeline, bind_group) =
            Renderer::create_pipeline(&sc_desc, device, queue, &shader, app);

        let mut example = Renderer {
            bundle: None, // bundle will be initialized bellow
            shader,
            pipeline,
            multisampled_framebuffer, // there should be nothing in here yet
            vertex_buffer,
            vertex_count,
            sample_count,
            matrix_bind_group: bind_group,
            rebuild_bundle: false, // wether the bundle and with it the vertex buffer is rebuilt every frame
            sc_desc: sc_desc.clone(),
            egui_renderer: EguiRenderer::init(device, sc_desc.format),
        };
        example.create_bundle(device, queue, app);
        example
    }

    fn generate_matrix(aspect_ratio: f32) -> cgmath::Matrix4<f32> {
        let mx_projection = cgmath::ortho(-aspect_ratio, aspect_ratio, -1.0, 1.0, 0., 1.);
        let mx_correction = crate::framework::OPENGL_TO_WGPU_MATRIX;
        mx_correction * mx_projection //* mx_view
    }

    pub fn resize(
        &mut self,
        sc_desc: &SwapChainDescriptor,
        device: &Device,
        queue: &Queue,
        app: &mut LightGarden,
    ) {
        self.sc_desc = sc_desc.clone();
        let (pipeline, bind_group) =
            Renderer::create_pipeline(&self.sc_desc, device, queue, &self.shader, app);
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
        self.multisampled_framebuffer =
            Renderer::create_multisampled_framebuffer(device, sc_desc, self.sample_count);
    }

    pub fn render_to_texture(&mut self, device: &Device, queue: &Queue) {}

    pub fn render_texture(&mut self) {}

    pub fn render(
        &mut self,
        frame: &SwapChainTexture,
        device: &Device,
        queue: &Queue,
        gui: &mut Gui,
    ) {
        let vb = gui.app.trace_all();
        // self.update_vertex_buffer_with_line_strips(device, &vb);
        self.update_vertex_buffer(device, &vb);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

        {
            // setup render pass
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui_rpass: RenderPassDescriptor"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if gui.app.recreate_pipeline {
                let (pipeline, bind_group) = Renderer::create_pipeline(
                    &self.sc_desc,
                    device,
                    queue,
                    &self.shader,
                    &mut gui.app,
                );
                self.pipeline = pipeline;
                self.matrix_bind_group = bind_group;
            }
            rpass.set_bind_group(0, &self.matrix_bind_group, &[]);
            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // slot 0
            rpass.draw(0..self.vertex_count, 0..1); // vertex range, instance range

            // egui renders here
        }
        queue.submit(iter::once(encoder.finish()));
        self.egui_renderer
            .render(device, queue, &self.sc_desc, &frame.view, gui);
    }
}
