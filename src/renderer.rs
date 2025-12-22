use crate::light_garden::LightGarden;
use crate::sub_render_pass::SubRenderPass;
use crate::texture_renderer::{RENDER_TEXTURE_FORMAT, TextureRenderer};
use bytemuck::{Pod, Zeroable};
use egui::FullOutput;
use egui_wgpu::ScreenDescriptor;
use half::f16;
use image::save_buffer_with_format;
use std::iter;
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
    sub_rpass_lines: SubRenderPass,
    sub_rpass_triangles: SubRenderPass,
    egui_rpass: egui_wgpu::Renderer,
    texture_renderer: TextureRenderer,
    surface_config: SurfaceConfiguration,
    pub make_screenshot: bool,
}

impl Renderer {
    pub fn init(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue, // we might need to meddle with the command queue
        app: &mut LightGarden,
    ) -> Self {
        use std::borrow::Cow;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer: wgsl shader module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let sub_rpass_lines = SubRenderPass::new(
            surface_config,
            device,
            queue,
            &shader,
            app,
            PrimitiveTopology::LineList,
        );
        let sub_rpass_triangles = SubRenderPass::new(
            surface_config,
            device,
            queue,
            &shader,
            app,
            PrimitiveTopology::TriangleList,
        );

        let texture_renderer =
            TextureRenderer::init(device, surface_config, app.color_state_descriptor.clone());

        let egui_rpass = egui_wgpu::Renderer::new(device, surface_config.format, None, 1, false);

        Renderer {
            shader,
            sub_rpass_lines,
            sub_rpass_triangles,
            egui_rpass,
            texture_renderer,
            surface_config: surface_config.clone(),
            make_screenshot: false,
        }
    }

    fn recreate_pipelines(&mut self, device: &Device, queue: &Queue, app: &mut LightGarden) {
        app.recreate_pipelines = false;
        self.sub_rpass_lines.recreate_pipeline(
            &self.surface_config,
            device,
            queue,
            &self.shader,
            app,
        );
        self.sub_rpass_triangles.recreate_pipeline(
            &self.surface_config,
            device,
            queue,
            &self.shader,
            app,
        );
    }

    fn recreate_projection_bind_groups(
        &mut self,
        device: &Device,
        queue: &Queue,
        app: &mut LightGarden,
    ) {
        self.sub_rpass_lines.recreate_projection_bind_group(
            device,
            queue,
            &self.surface_config,
            app,
        );
        self.sub_rpass_triangles.recreate_projection_bind_group(
            device,
            queue,
            &self.surface_config,
            app,
        );
    }

    pub fn generate_matrix(aspect_ratio: f32) -> cgmath::Matrix4<f32> {
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
        self.recreate_projection_bind_groups(device, queue, app);
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
            TexelCopyTextureInfo {
                texture: &self.texture_renderer.render_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            bytemuck::cast_slice(black.as_slice()),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.surface_config.width * 4 * 4),
                rows_per_image: Some(self.surface_config.height),
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
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.sub_rpass_lines.render(&mut rpass);
            self.sub_rpass_triangles.render(&mut rpass);
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
            view_formats: &[],
        });
        let view = &texture.create_view(&TextureViewDescriptor::default());
        let mut screenshot_encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command screenshot_encoder"),
        });
        {
            let mut rpass = screenshot_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass screenshot: RenderPassDescriptor"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.sub_rpass_lines.render(&mut rpass);
            self.sub_rpass_triangles.render(&mut rpass);
        }
        let copy_wrapper = TexelCopyTextureInfo {
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
        let copy_buffer = TexelCopyBufferInfo {
            buffer: &buff,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(self.surface_config.height),
            },
        };
        screenshot_encoder.copy_texture_to_buffer(copy_wrapper, copy_buffer, texture_extent);

        queue.submit(iter::once(screenshot_encoder.finish()));
        let buffer_slice = buff.slice(..);
        buffer_slice.map_async(MapMode::Read, |_arg| {});
        if device.poll(PollType::Wait).is_ok() {
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
                image::ColorType::Rgba8,
                image::ImageFormat::Jpeg,
            ) {
                Ok(()) => {}
                Err(e) => {
                    println!("Error: could not make screenshot");
                    println!("Message: {e}");
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
        app: &mut LightGarden,
        output: FullOutput,
        context: &egui::Context,
        scale_factor: f32,
    ) {
        self.clear_render_texture(queue);
        self.render_to_texture(encoder);
        if app.recreate_pipelines {
            let (pipeline, _bind_group_layout, bind_group, _sampler) =
                TextureRenderer::create_pipeline(
                    device,
                    &self.surface_config,
                    &self.texture_renderer.shader,
                    &self.texture_renderer.render_texture,
                    app.color_state_descriptor.clone(),
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

        {
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            let clipped_primitives = context.tessellate(output.shapes, 1.0);

            // Upload all resources for the GPU.
            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.surface_config.width, self.surface_config.height],
                pixels_per_point: scale_factor,
            };

            for (id, image_delta) in &output.textures_delta.set {
                self.egui_rpass
                    .update_texture(device, queue, *id, image_delta);
            }
            self.egui_rpass.update_buffers(
                device,
                queue,
                encoder,
                &clipped_primitives,
                &screen_descriptor,
            );
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass: RenderPassDescriptor"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
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
            self.egui_rpass.render(
                &mut rpass.forget_lifetime(),
                &clipped_primitives,
                &screen_descriptor,
            );
            for id in &output.textures_delta.free {
                self.egui_rpass.free_texture(id);
            }
        }
    }

    pub fn render(
        &mut self,
        frame: &SurfaceTexture,
        device: &Device,
        queue: &Queue,
        app: &mut LightGarden,
        output: FullOutput,
        context: &egui::Context,
        scale_factor: f32,
    ) {
        let render_result = app.draw();
        self.sub_rpass_lines
            .update_vertex_buffer(device, &render_result.lines);
        self.sub_rpass_triangles
            .update_vertex_buffer(device, &render_result.triangles);
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });
        if app.recreate_pipelines {
            self.recreate_pipelines(device, queue, app);
        }
        if app.get_render_to_texture() {
            self.render_texture(
                device,
                queue,
                &mut encoder,
                frame,
                app,
                output,
                context,
                scale_factor,
            );
        } else {
            {
                let view = frame.texture.create_view(&TextureViewDescriptor::default());
                let clipped_primitives = context.tessellate(output.shapes, 1.0);
                // Upload all resources for the GPU.
                let screen_descriptor = ScreenDescriptor {
                    size_in_pixels: [self.surface_config.width, self.surface_config.height],
                    pixels_per_point: scale_factor,
                };
                for (id, image_delta) in &output.textures_delta.set {
                    self.egui_rpass
                        .update_texture(device, queue, *id, image_delta);
                }
                self.egui_rpass.update_buffers(
                    device,
                    queue,
                    &mut encoder,
                    &clipped_primitives,
                    &screen_descriptor,
                );
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("rpass: RenderPassDescriptor"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(wgpu::Color::BLACK),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                self.sub_rpass_lines.render(&mut rpass);
                self.sub_rpass_triangles.render(&mut rpass);

                self.egui_rpass.render(
                    &mut rpass.forget_lifetime(),
                    &clipped_primitives,
                    &screen_descriptor,
                );
                for id in &output.textures_delta.free {
                    self.egui_rpass.free_texture(id);
                }
            }
        }
        queue.submit(iter::once(encoder.finish()));
    }
}
