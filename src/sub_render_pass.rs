use crate::light_garden::light::Color;
use crate::light_garden::LightGarden;
use crate::renderer::{Renderer, Vertex};
use crate::texture_renderer::RENDER_TEXTURE_FORMAT;
use collision2d::geo::*;
use wgpu::util::DeviceExt;
use wgpu::*;

pub struct SubRenderPass {
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub buffer_length: usize,
    pub matrix_bind_group: BindGroup,
    pub topology: PrimitiveTopology,
}

impl SubRenderPass {
    pub fn new(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        app: &mut LightGarden,
        topology: PrimitiveTopology,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 0,
            mapped_at_creation: true,
            usage: BufferUsages::VERTEX,
        });
        let (pipeline, matrix_bind_group) =
            SubRenderPass::create_pipeline(surface_config, device, queue, shader, app, topology);
        SubRenderPass {
            pipeline,
            vertex_buffer,
            buffer_length: 0,
            matrix_bind_group,
            topology,
        }
    }

    fn create_pipeline(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        app: &mut LightGarden,
        primitive_topology: PrimitiveTopology,
    ) -> (RenderPipeline, BindGroup) {
        app.recreate_pipelines = false;
        // layout for the projection matrix
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Renderer: bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(64),
                },
                count: None,
            }],
        });

        // create the projection matrix
        let aspect = surface_config.width as f32 / surface_config.height as f32;
        let mx = Renderer::generate_matrix(aspect);
        let mx_ref: &[f32; 16] = mx.as_ref();
        let mx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("u_Transform"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // set new canvas bounds
        let new_canvas_bounds = Rect::from_tlbr(1., -aspect as f64, -1., aspect as f64);
        app.tracer.resize(&new_canvas_bounds);
        app.drawer.resize(&new_canvas_bounds);

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
                    targets: &[Some(app.color_state_descriptor.clone())],
                }),
                // render lines
                primitive: PrimitiveState {
                    topology: primitive_topology,
                    front_face: FrontFace::Cw,
                    ..Default::default()
                },
                depth_stencil: None,
                // no multisample
                multisample: MultisampleState {
                    ..Default::default()
                },
                multiview: None,
            }),
            bind_group,
        )
    }

    pub fn recreate_pipeline(
        &mut self,
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        app: &mut LightGarden,
    ) {
        let (pipeline, bind_group) =
            SubRenderPass::create_pipeline(surface_config, device, queue, shader, app, self.topology);
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
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
        self.buffer_length = vertex_data.len();
    }

    pub fn render<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_bind_group(0, &self.matrix_bind_group, &[]);
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // slot 0
        rpass.draw(0..(self.buffer_length as u32), 0..1); // vertex range, instance range
    }
}
