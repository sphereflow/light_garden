use crate::egui_renderer::EguiRenderer;
use crate::gui::Gui;
use crate::light_garden::light::Color;
use crate::light_garden::LightGarden;
use crate::texture_renderer::{TextureRenderer, RENDER_TEXTURE_FORMAT};
use bytemuck::{Pod, Zeroable};
use collision2d::geo::*;
use egui::ClippedMesh;
use half::f16;
use image::save_buffer_with_format;
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
    surface_config: SurfaceConfiguration,
    pub egui_renderer: EguiRenderer,
    pub make_screenshot: bool,
}

impl Renderer {
    fn create_pipeline(
        surface_config: &SurfaceConfiguration,
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
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });

        // create the projection matrix
        let aspect = surface_config.width as f32 / surface_config.height as f32;
        let mx = Self::generate_matrix(aspect);
        let mx_ref: &[f32; 16] = mx.as_ref();
        let mx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("u_Transform"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
            app.color_state_descriptor.format = surface_config.format;
        }

        (
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("render pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
                    }],
                },
                fragment: Some(FragmentState {
                    module: shader,
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
            usage: BufferUsages::VERTEX,
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
            usage: BufferUsages::VERTEX,
        });
        self.vertex_count = vertex_data.len() as u32;
        self.rebuild_bundle = true;
    }

    pub fn init(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue, // we might need to meddle with the command queue
        app: &mut LightGarden,
    ) -> Self {
        use std::borrow::Cow;
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Renderer: wgsl shader module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        // create the vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 0,
            mapped_at_creation: true,
            usage: BufferUsages::VERTEX,
        });
        let (pipeline, bind_group) =
            Renderer::create_pipeline(surface_config, device, queue, &shader, app);

        let texture_renderer =
            TextureRenderer::init(device, surface_config, app.color_state_descriptor.clone());

        Renderer {
            shader,
            pipeline,
            vertex_buffer,
            vertex_count: 0,
            matrix_bind_group: bind_group,
            rebuild_bundle: false, // wether the bundle and with it the vertex buffer is rebuilt every frame
            texture_renderer,
            surface_config: surface_config.clone(),
            egui_renderer: EguiRenderer::init(device, surface_config.format),
            make_screenshot: false,
        }
    }

    fn generate_matrix(aspect_ratio: f32) -> cgmath::Matrix4<f32> {
        let mx_projection = cgmath::ortho(-aspect_ratio, aspect_ratio, -1.0, 1.0, 0., 1.);
        let mx_correction = crate::framework::OPENGL_TO_WGPU_MATRIX;
        mx_correction * mx_projection //* mx_view
    }

    pub fn resize(
        &mut self,
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        app: &mut LightGarden,
    ) {
        self.surface_config = surface_config.clone();
        self.texture_renderer
            .generate_render_texture(device, &self.surface_config);

        let (pipeline, bind_group) =
            Renderer::create_pipeline(&self.surface_config, device, queue, &self.shader, app);
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
        self.texture_renderer
            .generate_render_texture(device, &self.surface_config);
    }

    fn clear_render_texture(&mut self, queue: &Queue) {
        let size = (self.surface_config.width * self.surface_config.height) as usize;
        let dimensions = Extent3d {
            width: self.surface_config.width,
            height: self.surface_config.height,
            depth_or_array_layers: 1,
        };
        let black: Vec<[f32; 4]> = vec![[0., 0., 0., 1.]; size];
        queue.write_texture(
            ImageCopyTexture {
                texture: &self.texture_renderer.render_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            bytemuck::cast_slice(black.as_slice()),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(self.surface_config.width * 4 * 4),
                rows_per_image: NonZeroU32::new(self.surface_config.height),
            },
            dimensions,
        );
    }

    fn render_to_texture(&mut self, encoder: &mut CommandEncoder) {
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
    }

    pub async fn make_screenshot(
        &mut self,
        path: String,
        device: &Device,
        queue: &Queue,
        render_to_texture: bool,
    ) {
        let texture_extent = Extent3d {
            width: self.surface_config.width,
            height: self.surface_config.height,
            depth_or_array_layers: 1,
        };
        let format;
        let unpadded_bytes_per_row;
        if render_to_texture {
            format = RENDER_TEXTURE_FORMAT;
            unpadded_bytes_per_row = 8 * self.surface_config.width;
        } else {
            format = TextureFormat::Bgra8UnormSrgb;
            unpadded_bytes_per_row = 4 * self.surface_config.width;
        }
        let texture = device.create_texture(&TextureDescriptor {
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: None,
        });
        let view = &texture.create_view(&TextureViewDescriptor::default());
        let mut screenshot_encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command screenshot_encoder"),
        });
        {
            let mut rpass = screenshot_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass screenshot: RenderPassDescriptor"),
                color_attachments: &[RenderPassColorAttachment {
                    view,
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
        let copy_wrapper = ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        };
        let align = COPY_BYTES_PER_ROW_ALIGNMENT;
        let is_aligned = match (unpadded_bytes_per_row) % align {
            0 => 0,
            _ => 1,
        };
        let padded_bytes_per_row = (((unpadded_bytes_per_row) / align) + is_aligned) * align;
        let buff_desc = BufferDescriptor {
            label: Some("screen shot buffer descriptor"),
            mapped_at_creation: false,
            size: (padded_bytes_per_row * self.surface_config.height) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        };
        let buff: Buffer = device.create_buffer(&buff_desc);
        let copy_buffer = ImageCopyBuffer {
            buffer: &buff,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(padded_bytes_per_row),
                rows_per_image: NonZeroU32::new(self.surface_config.height),
            },
        };
        screenshot_encoder.copy_texture_to_buffer(copy_wrapper, copy_buffer, texture_extent);

        queue.submit(iter::once(screenshot_encoder.finish()));
        let buffer_slice = buff.slice(..);
        let bytes_future = buffer_slice.map_async(MapMode::Read);
        device.poll(Maintain::Wait);

        if let Ok(()) = bytes_future.await {
            let padded_buffer = buffer_slice.get_mapped_range();
            let mut bufvec = Vec::new();
            for padded in padded_buffer.chunks(padded_bytes_per_row as usize) {
                if render_to_texture {
                    for pixel in padded[..(unpadded_bytes_per_row) as usize].chunks_exact(8) {
                        bufvec.extend_from_slice(&Renderer::convert_pixel_rgbaf16_to_bgra8(pixel));
                    }
                } else {
                    bufvec.extend_from_slice(&padded[..(unpadded_bytes_per_row) as usize])
                }
            }
            match save_buffer_with_format(
                path,
                &bufvec,
                self.surface_config.width,
                self.surface_config.height,
                image::ColorType::Bgra8,
                image::ImageFormat::Jpeg,
            ) {
                Ok(()) => {}
                Err(e) => {
                    println!("Error: could not make screenshot");
                    println!("Message: {}", e);
                }
            };
            drop(padded_buffer);
        }
        buff.unmap();
    }

    fn convert_pixel_rgbaf16_to_bgra8(pixel: &[u8]) -> [u8; 4] {
        let r = &pixel[0..2];
        let g = &pixel[2..4];
        let b = &pixel[4..6];
        let a = &pixel[6..];
        let nr = Renderer::f16_to_u8(r);
        let ng = Renderer::f16_to_u8(g);
        let nb = Renderer::f16_to_u8(b);
        let na = Renderer::f16_to_u8(a);
        [nb, ng, nr, na]
    }

    fn f16_to_u8(half: &[u8]) -> u8 {
        let f: f32 = f16::from_le_bytes([half[0], half[1]]).into();
        ((f.powf(1. / 2.2)) * 255.) as u8
    }

    fn render_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        frame: &SurfaceTexture,
        gui: &mut Gui,
        clipped_meshes: &[ClippedMesh],
    ) {
        self.clear_render_texture(queue);
        self.render_to_texture(encoder);
        if gui.app.recreate_pipeline {
            let (pipeline, _bind_group_layout, bind_group, _sampler) =
                TextureRenderer::create_pipeline(
                    device,
                    &self.surface_config,
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
            encoder,
            &self.surface_config,
            &frame.texture.create_view(&TextureViewDescriptor::default()),
            gui,
            clipped_meshes,
        );

        {
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass: RenderPassDescriptor"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
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
            // vertex range, instance range
            rpass.draw_indexed(0..self.texture_renderer.index_buffer_size, 0, 0..1);
        }
    }

    pub fn render(
        &mut self,
        frame: &SurfaceTexture,
        device: &Device,
        queue: &Queue,
        gui: &mut Gui,
        clipped_meshes: &[ClippedMesh],
    ) {
        let vb = gui.app.draw();
        // self.update_vertex_buffer_with_line_strips(device, &vb);
        self.update_vertex_buffer(device, &vb);
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

        if gui.app.recreate_pipeline {
            let (pipeline, bind_group) = Renderer::create_pipeline(
                &self.surface_config,
                device,
                queue,
                &self.shader,
                &mut gui.app,
            );
            self.pipeline = pipeline;
            self.matrix_bind_group = bind_group;
        }

        if gui.app.get_render_to_texture() {
            self.render_texture(device, queue, &mut encoder, frame, gui, clipped_meshes);
        } else {
            {
                let view = frame.texture.create_view(&TextureViewDescriptor::default());
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("rpass: RenderPassDescriptor"),
                    color_attachments: &[RenderPassColorAttachment {
                        view: &view,
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
        }

        self.egui_renderer.render(
            device,
            queue,
            &mut encoder,
            &self.surface_config,
            &frame.texture.create_view(&TextureViewDescriptor::default()),
            gui,
            clipped_meshes,
        );
        queue.submit(iter::once(encoder.finish()));
    }
}
