use crate::framework::cast_slice;
use crate::gui::Gui;
use bytemuck::{Pod, Zeroable};
use egui::*;
use std::sync::Arc;
use std::num::NonZeroU32;
use wgpu::util::DeviceExt;
use wgpu::*;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct UniformBuffer {
    screen_size: [f32; 2],
}
unsafe impl Pod for UniformBuffer {}
unsafe impl Zeroable for UniformBuffer {}

#[derive(Debug)]
struct SizedBuffer {
    buffer: Buffer,
    size: usize,
}

/// Enum for selecting the right buffer type.
#[derive(Debug)]
enum BufferType {
    Uniform,
    Index,
    Vertex,
}

pub struct EguiRenderer {
    // egui stuff
    /// used to communicate the screen space coordinates to the shaders
    render_pipeline: RenderPipeline,
    uniform_buffer: SizedBuffer,
    uniform_bind_group: BindGroup,
    vertex_buffers: Vec<SizedBuffer>,
    index_buffers: Vec<SizedBuffer>,
    texture_version: Option<u64>,
    texture_bind_group_layout: BindGroupLayout,
    texture_bind_group: Option<BindGroup>,
    next_user_texture_id: u64,
    pending_user_textures: Vec<(u64, egui::Texture)>,
    user_textures: Vec<Option<BindGroup>>,
}

impl EguiRenderer {
    pub fn init(device: &Device, output_format: TextureFormat) -> Self {
        use std::borrow::Cow;
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("egui: wgsl shader module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("egui.wgsl"))),
        });

        // eguis initialization
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("egui: uniform_buffer"),
            contents: bytemuck::cast_slice(&[UniformBuffer {
                screen_size: [0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // uniform buffer for screen size
        let uniform_buffer = SizedBuffer {
            buffer: uniform_buffer,
            size: std::mem::size_of::<UniformBuffer>(),
        };

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui: uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: None,
                        ty: wgpu::BufferBindingType::Uniform,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui: uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &uniform_buffer.buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui: texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("egui_pipeline_layout"),
            // layout => [set 0: uniform bind group, set 1: texture bind group]
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("egui_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                entry_point: "vs_main",
                module: &shader,
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 5 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    // 0: vec2 position
                    // 1: vec2 texture coordinates
                    // 2: uint color
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                conservative: false,
                cull_mode: None,
                unclipped_depth: false,
                front_face: wgpu::FrontFace::default(),
                polygon_mode: wgpu::PolygonMode::default(),
                strip_index_format: None,
            },
            multiview: None,
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                count: 1,
                mask: !0,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
        });

        EguiRenderer {
            render_pipeline,
            vertex_buffers: Vec::new(),
            index_buffers: Vec::new(),
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            texture_version: None,
            texture_bind_group: None,
            next_user_texture_id: 0,
            pending_user_textures: Vec::new(),
            user_textures: Vec::new(),
        }
    }

    /// Uploads the uniform, vertex and index data used by the render pass. Should be called before `execute()`.
    pub fn update_buffers(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &wgpu::SurfaceConfiguration,
        clipped_meshes: &[ClippedMesh],
        scale_factor: f32,
    ) {
        let index_size = self.index_buffers.len();
        let vertex_size = self.vertex_buffers.len();

        let (logical_width, logical_height) = (
            surface_config.width as f32 / scale_factor,
            surface_config.height as f32 / scale_factor,
        );

        self.update_buffer(
            device,
            queue,
            BufferType::Uniform,
            0,
            bytemuck::cast_slice(&[UniformBuffer {
                screen_size: [logical_width as f32, logical_height as f32],
            }]),
        );

        for (i, ClippedMesh(_rect, triangles)) in clipped_meshes.iter().enumerate() {
            let data: &[u8] = bytemuck::cast_slice(&triangles.indices);
            if i < index_size {
                self.update_buffer(device, queue, BufferType::Index, i, data)
            } else {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("egui_index_buffer"),
                    contents: data,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
                let s_buffer = SizedBuffer {
                    buffer,
                    size: data.len(),
                };
                self.index_buffers.push(s_buffer);
            }

            let data: &[u8] = cast_slice(&triangles.vertices);
            if i < vertex_size {
                self.update_buffer(device, queue, BufferType::Vertex, i, data)
            } else {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("egui_vertex_buffer"),
                    contents: data,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
                let s_buffer = SizedBuffer {
                    buffer,
                    size: data.len(),
                };

                self.vertex_buffers.push(s_buffer);
            }
        }
    }

    /// Updates the buffers used by egui. Will properly re-size the buffers if needed.
    fn update_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer_type: BufferType,
        index: usize,
        data: &[u8],
    ) {
        let (buffer, storage, name) = match buffer_type {
            BufferType::Index => (&mut self.index_buffers[index], BufferUsages::INDEX, "index"),
            BufferType::Vertex => (
                &mut self.vertex_buffers[index],
                BufferUsages::VERTEX,
                "vertex",
            ),
            BufferType::Uniform => (&mut self.uniform_buffer, BufferUsages::UNIFORM, "uniform"),
        };

        if data.len() > buffer.size {
            buffer.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("egui_{}_buffer", name).as_str()),
                contents: bytemuck::cast_slice(data),
                usage: storage | wgpu::BufferUsages::COPY_DST,
            });
        } else {
            queue.write_buffer(&buffer.buffer, 0, data);
        }
    }

    pub fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: Arc<egui::Texture>,
    ) {
        // Don't update the texture if it hasn't changed.
        if self.texture_version == Some(texture.version) {
            return;
        }
        // we need to convert the texture into rgba format
        let egui_texture = egui::Texture {
            version: texture.version,
            width: texture.width,
            height: texture.height,
            pixels: texture
                .pixels
                .iter()
                .flat_map(|&p| Vec::from(egui::epaint::Color32::from_white_alpha(p).to_array()))
                .collect(),
        };
        let bind_group = self.texture_to_wgpu(device, queue, &egui_texture, "egui");
        self.texture_version = Some(egui_texture.version);
        self.texture_bind_group = Some(bind_group);
    }

    /// Updates the user textures that the app allocated. Should be called before `execute()`.
    pub fn update_user_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let pending_user_textures = std::mem::take(&mut self.pending_user_textures);
        for (id, texture) in pending_user_textures {
            let bind_group = self.texture_to_wgpu(
                device,
                queue,
                &texture,
                format!("user_texture{}", id).as_str(),
            );
            self.user_textures.push(Some(bind_group));
        }
    }

    fn texture_to_wgpu(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &egui::Texture,
        label: &str,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: texture.width as u32,
            height: texture.height as u32,
            depth_or_array_layers: 1,
        };

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(format!("{}_texture", label).as_str()),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            texture.pixels.as_slice(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new((texture.pixels.len() / texture.height) as u32),
                rows_per_image: NonZeroU32::new(texture.height as u32),
            },
            size,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("egui: texture_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(format!("{}_texture_bind_group", label).as_str()),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        bind_group
    }

    fn get_texture_bind_group(&self, texture_id: egui::TextureId) -> &wgpu::BindGroup {
        match texture_id {
            egui::TextureId::Egui => self
                .texture_bind_group
                .as_ref()
                .expect("egui texture was not set before the first draw"),
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                self.user_textures
                    .get(id)
                    .unwrap_or_else(|| panic!("user texture {} not found", id))
                    .as_ref()
                    .unwrap_or_else(|| panic!("user texture {} freed", id))
            }
        }
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        surface_config: &SurfaceConfiguration,
        color_attachment: &TextureView,
        gui: &mut Gui,
        clipped_meshes: &[ClippedMesh],
    ) {
        self.update_texture(device, queue, gui.platform.context().texture());
        self.update_user_textures(device, queue);
        self.update_buffers(
            device,
            queue,
            surface_config,
            clipped_meshes,
            gui.scale_factor as f32,
        );
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: color_attachment,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            let scale_factor = gui.scale_factor;
            let physical_width = surface_config.width;
            let physical_height = surface_config.height;

            for ((ClippedMesh(clip_rect, triangles), vertex_buffer), index_buffer) in clipped_meshes
                .iter()
                .zip(self.vertex_buffers.iter())
                .zip(self.index_buffers.iter())
            {
                if !EguiRenderer::set_clip_rect(
                    clip_rect,
                    scale_factor,
                    physical_width,
                    physical_height,
                    &mut rpass,
                ) {
                    continue;
                }
                rpass.set_bind_group(1, self.get_texture_bind_group(triangles.texture_id), &[]);

                rpass.set_index_buffer(index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
                rpass.draw_indexed(0..triangles.indices.len() as u32, 0, 0..1);
            }
        }
    }

    /// returns if the area of the clip rect is non zero
    fn set_clip_rect(
        clip_rect: &egui::Rect,
        scale_factor: f32,
        physical_width: u32,
        physical_height: u32,
        rpass: &mut wgpu::RenderPass,
    ) -> bool {
        // Transform clip rect to physical pixels.
        let clip_min_x = scale_factor * clip_rect.min.x;
        let clip_min_y = scale_factor * clip_rect.min.y;
        let clip_max_x = scale_factor * clip_rect.max.x;
        let clip_max_y = scale_factor * clip_rect.max.y;

        // Make sure clip rect can fit within an `u32`.
        let clip_min_x = clip_min_x.clamp(0., physical_width as f32);
        let clip_min_y = clip_min_y.clamp(0., physical_height as f32);
        let clip_max_x = clip_max_x.clamp(clip_min_x, physical_width as f32);
        let clip_max_y = clip_max_y.clamp(clip_min_y, physical_height as f32);

        let clip_min_x = clip_min_x.round() as u32;
        let clip_min_y = clip_min_y.round() as u32;
        let clip_max_x = clip_max_x.round() as u32;
        let clip_max_y = clip_max_y.round() as u32;

        let width = (clip_max_x - clip_min_x).max(1);
        let height = (clip_max_y - clip_min_y).max(1);

        {
            // clip scissor rectangle to target size
            let x = clip_min_x.min(physical_width);
            let y = clip_min_y.min(physical_height);
            let width = width.min(physical_width - x);
            let height = height.min(physical_height - y);

            // skip rendering with zero-sized clip areas
            if width == 0 || height == 0 {
                return false;
            }

            rpass.set_scissor_rect(x, y, width, height);
            true
        }
    }
}

impl epi::TextureAllocator for EguiRenderer {
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[egui::Color32],
    ) -> egui::TextureId {
        let id = self.next_user_texture_id;
        self.next_user_texture_id += 1;

        let mut pixels = vec![0u8; srgba_pixels.len() * 4];
        for (target, given) in pixels.chunks_exact_mut(4).zip(srgba_pixels.iter()) {
            target.copy_from_slice(&given.to_array());
        }

        let (width, height) = size;
        self.pending_user_textures.push((
            id,
            egui::Texture {
                version: 0,
                width,
                height,
                pixels,
            },
        ));

        egui::TextureId::User(id)
    }

    fn free(&mut self, id: egui::TextureId) {
        if let egui::TextureId::User(id) = id {
            self.user_textures
                .get_mut(id as usize)
                .and_then(|option| option.take());
        }
    }
}
