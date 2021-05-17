use crate::egui_renderer::EguiRenderer;
use crate::gui::Gui;
use crate::light_garden::light::Color;
use crate::light_garden::LightGarden;
use crate::texture_renderer::{TextureRenderer, RENDER_TEXTURE_FORMAT};
use bytemuck::{Pod, Zeroable};
use collision2d::geo::*;
use egui::ClippedMesh;
use std::{iter, num::NonZeroU32};
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub _pos: [f32; 2],
    pub _color: [f32; 4],
    pub _tex_coord: [f32; 2],
}
unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

pub struct Renderer {
    shader: ShaderModule,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_count: u32,
    matrix_bind_group: BindGroup,
    rebuild_bundle: bool,
    texture_renderer: TextureRenderer,
    sc_desc: SwapChainDescriptor,
    pub egui_renderer: EguiRenderer,
}

impl Renderer {
    fn create_pipeline(
        sc_desc: &SwapChainDescriptor,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        app: &mut LightGarden,
    ) -> (RenderPipeline, BindGroup) {
        app.recreate_pipeline = false;
        // layout for the projection matrix
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Renderer: bind group layout"),
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

        // create the projection matrix
        let aspect = sc_desc.width as f32 / sc_desc.height as f32;
        let mx = Self::generate_matrix(aspect);
        let mx_ref: &[f32; 16] = mx.as_ref();
        let mx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("u_Transform"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        // set new canvas bounds
        app.tracer
            .resize(&Rect::from_tlbr(1., -aspect as f64, -1., aspect as f64));

        // write to the projection matix buffer
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("u_Transform"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &mx_buf,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        queue.write_buffer(&mx_buf, 0, bytemuck::cast_slice(mx_ref));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        if app.get_render_to_texture() {
            app.color_state_descriptor.format = RENDER_TEXTURE_FORMAT;
        } else {
            app.color_state_descriptor.format = TextureFormat::Bgra8UnormSrgb;
        }

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
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
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
                    front_face: FrontFace::Cw,
                    ..Default::default()
                },
                depth_stencil: None,
                // no multisample
                multisample: MultisampleState {
                    ..Default::default()
                },
            }),
            bind_group,
        )
    }

    pub fn update_vertex_buffer(&mut self, device: &Device, vertices: &[(P2, Color)]) {
        let vertex_data: Vec<Vertex> = vertices
            .iter()
            .map(|(p, color)| Vertex {
                _pos: [p.x as f32, p.y as f32],
                _color: *color,
                _tex_coord: [0., 0.],
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
        vertices: &[(Vec<P2>, Color)],
    ) {
        let mut vertex_data: Vec<Vertex> = Vec::with_capacity(vertices.len() * 2);
        for (line_strip, color) in vertices {
            for w in line_strip.windows(2) {
                vertex_data.push(Vertex {
                    _pos: [w[0].x as f32, w[0].y as f32],
                    _color: *color,
                    _tex_coord: [0., 0.],
                });
                vertex_data.push(Vertex {
                    _pos: [w[1].x as f32, w[1].y as f32],
                    _color: *color,
                    _tex_coord: [0., 0.],
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
        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match adapter.get_info().backend {
            wgpu::Backend::Metal | wgpu::Backend::Vulkan => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
            }
            _ => (), //TODO
        }

        use std::borrow::Cow;
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Renderer: wgsl shader module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
            flags,
        });

        // create the vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 0,
            mapped_at_creation: true,
            usage: BufferUsage::VERTEX,
        });
        let (pipeline, bind_group) =
            Renderer::create_pipeline(&sc_desc, device, queue, &shader, app);

        let texture_renderer =
            TextureRenderer::init(device, adapter, sc_desc, app.color_state_descriptor.clone());

        Renderer {
            shader,
            pipeline,
            vertex_buffer,
            vertex_count: 0,
            matrix_bind_group: bind_group,
            rebuild_bundle: false, // wether the bundle and with it the vertex buffer is rebuilt every frame
            texture_renderer,
            sc_desc: sc_desc.clone(),
            egui_renderer: EguiRenderer::init(device, adapter, sc_desc.format),
        }
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
        self.texture_renderer
            .generate_render_texture(device, &self.sc_desc);

        let (pipeline, bind_group) =
            Renderer::create_pipeline(&self.sc_desc, device, queue, &self.shader, app);
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
        self.texture_renderer
            .generate_render_texture(device, &self.sc_desc);
    }

    fn clear_render_texture(&mut self, queue: &Queue) {
        let size = (self.sc_desc.width * self.sc_desc.height) as usize;
        let dimensions = Extent3d {
            width: self.sc_desc.width,
            height: self.sc_desc.height,
            depth_or_array_layers: 1,
        };
        let black: Vec<[f32; 4]> = vec![[0., 0., 0., 1.]; size];
        queue.write_texture(
            ImageCopyTexture {
                texture: &self.texture_renderer.render_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            bytemuck::cast_slice(black.as_slice()),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(self.sc_desc.width * 4 * 4),
                rows_per_image: NonZeroU32::new(self.sc_desc.height),
            },
            dimensions,
        );
    }

    fn render_to_texture(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("render to texture command encoder"),
        });
        {
            let view = self
                .texture_renderer
                .render_texture
                .create_view(&TextureViewDescriptor::default());
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render to texture render pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                    resolve_target: None,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_bind_group(0, &self.matrix_bind_group, &[]);
            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // slot 0
            rpass.draw(0..self.vertex_count, 0..1); // vertex range, instance range
        }
        queue.submit(iter::once(encoder.finish()));
    }

    fn render_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        frame: &SwapChainTexture,
        gui: &mut Gui,
        clipped_meshes: &[ClippedMesh],
    ) {
        self.clear_render_texture(queue);
        self.render_to_texture(device, queue);
        if gui.app.recreate_pipeline {
            let (pipeline, _bind_group_layout, bind_group, _sampler) =
                TextureRenderer::create_pipeline(
                    device,
                    &self.sc_desc,
                    &self.texture_renderer.shader,
                    &self.texture_renderer.render_texture,
                    gui.app.color_state_descriptor.clone(),
                );
            self.texture_renderer.pipeline = pipeline;
            self.texture_renderer.bind_group = bind_group;
        }
        // the bind group must be recreated every frame
        self.texture_renderer.bind_group = TextureRenderer::create_bind_group(
            device,
            &self.texture_renderer.bind_group_layout,
            &self.texture_renderer.render_texture,
            &self.texture_renderer.sampler,
        );

        self.egui_renderer.render(
            device,
            queue,
            &self.sc_desc,
            &frame.view,
            gui,
            clipped_meshes,
        );

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass: RenderPassDescriptor"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.texture_renderer.pipeline);
            rpass.set_bind_group(0, &self.texture_renderer.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.texture_renderer.background_quad_buffer.slice(..)); // slot 0
            rpass.set_index_buffer(
                self.texture_renderer.background_quad_index_buffer.slice(..),
                IndexFormat::Uint16,
            );
            rpass.draw_indexed(0..self.texture_renderer.index_buffer_size, 0, 0..1);
            // vertex range, instance range
        }
        queue.submit(iter::once(encoder.finish()));
        
        self.egui_renderer.render(
            device,
            queue,
            &self.sc_desc,
            &frame.view,
            gui,
            clipped_meshes,
        );

    }

    pub fn render(
        &mut self,
        frame: &SwapChainTexture,
        device: &Device,
        queue: &Queue,
        gui: &mut Gui,
        clipped_meshes: &[ClippedMesh],
    ) {
        let vb = gui.app.draw();
        // self.update_vertex_buffer_with_line_strips(device, &vb);
        self.update_vertex_buffer(device, &vb);

        if gui.app.recreate_pipeline {
            let (pipeline, bind_group) =
                Renderer::create_pipeline(&self.sc_desc, device, queue, &self.shader, &mut gui.app);
            self.pipeline = pipeline;
            self.matrix_bind_group = bind_group;
        }

        if gui.app.get_render_to_texture() {
            self.render_texture(device, queue, frame, gui, clipped_meshes);
        } else {
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

            {
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("rpass: RenderPassDescriptor"),
                    color_attachments: &[RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });
                rpass.set_bind_group(0, &self.matrix_bind_group, &[]);
                rpass.set_pipeline(&self.pipeline);
                rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // slot 0
                rpass.draw(0..self.vertex_count, 0..1); // vertex range, instance range
            }
            queue.submit(iter::once(encoder.finish()));

            self.egui_renderer.render(
                device,
                queue,
                &self.sc_desc,
                &frame.view,
                gui,
                clipped_meshes,
            );

        }
    }
}
