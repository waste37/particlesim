use wgpu::util::DeviceExt;
use rand::RngExt;
use bytemuck::{Pod, Zeroable};

const WORKGROUP_SIZE: u32 = 64;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
}

pub struct ParticleSimDescriptor<'a> {
    pub device: &'a wgpu::Device,
    pub surface_config: &'a wgpu::SurfaceConfiguration,
    pub num_particles: u32
}

pub struct ParticleSim {
    update_pipeline: wgpu::ComputePipeline,
    particle_bind_groups: Vec<wgpu::BindGroup>,
    particle_buffers: Vec<wgpu::Buffer>,
    draw_pipeline: wgpu::RenderPipeline,
    workgroup_count: u32,
    num_particles: u32,
    frame_num: usize,
}

impl ParticleSim {
    pub fn new(descriptor: &ParticleSimDescriptor) -> Self {
        let device = descriptor.device;
        let num_particles = descriptor.num_particles;


        let update_bind_group_layout = ParticleSim::create_update_bind_group_layout(device, num_particles);
        let update_pipeline = ParticleSim::create_update_pipeline(device, &update_bind_group_layout);
        let particle_buffers = ParticleSim::create_particle_buffers(device, num_particles);
        let particle_bind_groups = 
            ParticleSim::create_particle_bind_groups(device, &update_bind_group_layout, &particle_buffers);

        let draw_pipeline = ParticleSim::create_draw_pipeline(device, descriptor.surface_config);
        let workgroup_count = descriptor.num_particles.div_ceil(WORKGROUP_SIZE);

        Self {
            update_pipeline,
            particle_bind_groups,
            particle_buffers,
            draw_pipeline,
            workgroup_count,
            num_particles,
            frame_num: 0
        }
    }


    pub fn update_particles(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.update_pipeline);
        cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
        cpass.dispatch_workgroups(self.workgroup_count, 1, 1);
    }

    pub fn draw_particles(&mut self, encoder: &mut wgpu::CommandEncoder, surface_texture: &wgpu::SurfaceTexture) {
        let view = surface_texture.texture.create_view(&Default::default());

        let mut rpass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        rpass.set_pipeline(&self.draw_pipeline);
        rpass.set_vertex_buffer(0, self.particle_buffers[(self.frame_num + 1) % 2].slice(..));
        rpass.draw(0..2, 0..self.num_particles);
        self.frame_num += 1;
    }
}

impl ParticleSim {
    fn create_update_bind_group_layout(device: &wgpu::Device, num_particles: u32) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("update_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((num_particles * 16) as _),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((num_particles * 16) as _),
                    },
                    count: None,
                },
            ],
        }) 
    }

    fn create_update_pipeline(device: &wgpu::Device, 
        bind_group_layout: &wgpu::BindGroupLayout) -> wgpu::ComputePipeline 
    {
        let shader = device.create_shader_module(wgpu::include_wgsl!("update.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("update"),
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("update_pipeline"), 
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    fn create_particle_buffers(device: &wgpu::Device, num_particles: u32) -> Vec::<wgpu::Buffer> {
        let mut initial_particles = vec![
            Particle{ position: [0.0f32, 0.0f32], velocity: [0.0f32, 0.0f32] }; 
            num_particles as usize
        ];

        for particle in &mut initial_particles {
            particle.position[0] = rand::rng().random_range(-1.0..1.0); 
            particle.position[1] = rand::rng().random_range(-1.0..1.0); 
        }

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();

        for i in 0..2 {
            particle_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Particle Buffer {i}")),
                    contents: bytemuck::cast_slice(&initial_particles),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        particle_buffers
    }

    fn create_particle_bind_groups(
        device: &wgpu::Device, 
        bind_group_layout: &wgpu::BindGroupLayout, 
        particle_buffers: &Vec::<wgpu::Buffer>,
    ) -> Vec::<wgpu::BindGroup> {
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            particle_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                ],
                label: None,
            }));
        }

        particle_bind_groups
    }

    fn create_draw_pipeline(
        device: &wgpu::Device, 
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::include_wgsl!("draw.wgsl"));
        let pipeline_layout = device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("draw_pipeline"), 
            layout: Some(&pipeline_layout),
            vertex: wgpu:: VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 4 * 4,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("main_fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }
}

