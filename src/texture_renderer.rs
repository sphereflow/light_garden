use crate::renderer::Vertex;
use wgpu::util::DeviceExt;
use wgpu::*;

pub const RENDER_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub struct TextureRenderer {
    pub background_quad_buffer: Buffer,
    pub background_quad_index_buffer: Buffer,
    pub render_texture: Texture,
    pub shader: ShaderModule,
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub sampler: Sampler,
    pub index_buffer_size: u32,
}

impl TextureRenderer {
    pub fn init(
        device: &Device,
        adapter: &Adapter,
        sc_desc: &SwapChainDescriptor,
        color_state_descriptor: ColorTargetState,
    ) -> Self {
        let background_quad: Vec<Vertex> = vec![
            Vertex {
                // top left
                _pos: [-1., 1.],
                _color: [1.; 4],
                _tex_coord: [0., 0.],
            },
            Vertex {
                // top right
                _pos: [1., 1.],
                _color: [1.; 4],
                _tex_coord: [1., 0.],
            },
            Vertex {
                // bottom right
                _pos: [1., -1.],
                _color: [1.; 4],
                _tex_coord: [1., 1.],
            },
            Vertex {
                // bottom left
                _pos: [-1., -1.],
                _color: [1.; 4],
                _tex_coord: [0., 1.],
            },
        ];
        let background_quad_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("background quad buffer"),
            contents: bytemuck::cast_slice(&background_quad),
            usage: BufferUsage::VERTEX,
        });
        let indices = [0_u16, 1, 2, 0, 2, 3];
        let background_quad_index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("background quad index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsage::INDEX,
        });
        let index_buffer_size = indices.len() as u32;

        let dimensions = Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        };
        let render_texture = device.create_texture(&TextureDescriptor {
            label: Some("render_texture"),
            size: dimensions,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: RENDER_TEXTURE_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        });

        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match adapter.get_info().backend {
            wgpu::Backend::Metal | wgpu::Backend::Vulkan => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
            }
            _ => (), //TODO
        }

        use std::borrow::Cow;
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("render to texture shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("render_to_texture.wgsl"))),
            flags,
        });

        let (pipeline, bind_group_layout, bind_group, sampler) = TextureRenderer::create_pipeline(
            device,
            sc_desc,
            &shader,
            &render_texture,
            color_state_descriptor,
        );

        TextureRenderer {
            background_quad_buffer,
            background_quad_index_buffer,
            render_texture,
            shader,
            pipeline,
            bind_group_layout,
            bind_group,
            sampler,
            index_buffer_size,
        }
    }

    pub fn create_pipeline(
        device: &Device,
        sc_desc: &SwapChainDescriptor,
        shader: &ShaderModule,
        render_texture: &Texture,
        mut color_state_descriptor: ColorTargetState,
    ) -> (RenderPipeline, BindGroupLayout, BindGroup, Sampler) {
        color_state_descriptor.format = sc_desc.format;

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = TextureRenderer::create_bind_group_layout(device);

        let bind_group = TextureRenderer::create_bind_group(
            device,
            &bind_group_layout,
            render_texture,
            &sampler,
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render to texture pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        (device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render to texture pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
                }],
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[color_state_descriptor],
            }),
            // render lines
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                ..Default::default()
            },
        }), bind_group_layout, bind_group, sampler)
    }

    fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render to texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn create_bind_group(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        render_texture: &Texture,
        sampler: &Sampler,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render to texture bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &render_texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn generate_render_texture(&mut self, device: &Device, sc_desc: &SwapChainDescriptor) {
        let dimensions = Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        };
        self.render_texture = device.create_texture(&TextureDescriptor {
            label: Some("render_texture"),
            size: dimensions,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: RENDER_TEXTURE_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        });
    }
}
