use wgpu::util::DeviceExt;
use rand::RngExt;
use bytemuck::{Pod, Zeroable};

const WORKGROUP_SIZE: u32 = 64;

/* Common CPU/GPU Particle definition */
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
}

/* Common CPU/GPU Uniforms */
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct SimulationUniforms {
    mouse_position: [f32; 2],
    mouse_down: i32,
    push: f32

}

//struct ParticleSimulationParams {
//}

#[derive(Clone, Copy, Default)]
pub struct ParticleSimulationInput {
    mouse_position: [f32; 2],
    mouse_down: bool
}

pub struct ParticleSystem {
    storage: ParticleStorage,
    simulation: ParticleSimulation,
    renderer: ParticleRenderer,
}

impl ParticleSystem {
    pub fn new(
        device: &wgpu::Device,
        render_format: wgpu::TextureFormat,
        particle_count: u32
    ) -> Self {
        let storage = ParticleStorage::new(device, particle_count);
        let simulation = ParticleSimulation::new(device, &storage.bind_group_layout, particle_count);
        let renderer = ParticleRenderer::new(device, render_format);

        Self {
            storage,
            simulation,
            renderer
        }
    }

    pub fn update(&mut self, encoder: &mut wgpu::CommandEncoder, dt: f32) {
        self.simulation.update(encoder, &self.storage, dt);
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.renderer.render(encoder, &mut self.storage, target);
        self.storage.swap_buffers();
    }
}

/* This struct is responsible for managing 
 * the gpu buffers containing the particles */
struct ParticleStorage {
    buffers: [wgpu::Buffer; 2],
    bind_groups: [wgpu::BindGroup; 2],
    bind_group_layout: wgpu::BindGroupLayout,
    particle_count: u32,
    current: usize
}

impl ParticleStorage {
    fn new(
        device: &wgpu::Device, 
        particle_count: u32, 
    ) -> Self {
        let bind_group_layout = 
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("particles_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((particle_count * 16) as _),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((particle_count * 16) as _),
                    },
                    count: None,
            }]}); 

        let mut initial_particles = vec![
            Particle{ position: [0.0f32, 0.0f32], velocity: [0.0f32, 0.0f32] }; 
            particle_count as usize
        ];

        for particle in &mut initial_particles {
            particle.position[0] = rand::rng().random_range(-1.0..1.0); 
            particle.position[1] = rand::rng().random_range(-1.0..1.0); 
        }

        let buffers: [wgpu::Buffer; 2] = std::array::from_fn(|i| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Particle Buffer {i}")),
                contents: bytemuck::cast_slice(&initial_particles),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            })
        });

        let bind_groups: [wgpu::BindGroup; 2] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers[(i + 1) % 2].as_entire_binding(),
                    },
                ],
                label: None,
            })
        });



        Self {
            buffers,
            bind_groups,
            bind_group_layout,
            particle_count,
            current: 0
        }
    }
    fn current_buffer(&self) -> &wgpu::Buffer {
        &self.buffers[self.current]
    }
    fn current_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_groups[self.current]
    }
    fn swap_buffers(&mut self) {
        self.current = 1 - self.current;
    }
}


/* This struct manages the GPU pipeline and creates the commands
 * needed to update the simulation and its parameters. */
struct ParticleSimulation {
    pipeline: wgpu::ComputePipeline,
    //parameters_buffer: wgpu::Buffer,
    workgroup_count: u32,
}

impl ParticleSimulation {
    fn new(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, particle_count: u32) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("update.wgsl"));
        let pipeline_layout = device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("update"),
                bind_group_layouts: &[Some(bind_group_layout)],
                immediate_size: 0
            });

        let pipeline = device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("update_pipeline"), 
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let workgroup_count = particle_count.div_ceil(WORKGROUP_SIZE);

        Self {
            pipeline,
            workgroup_count
        }
    }

    fn update(
        &self,
        encoder: &mut wgpu::CommandEncoder, 
        storage: &ParticleStorage, 
        dt: f32,
    ) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, storage.current_bind_group(), &[]);
        cpass.dispatch_workgroups(self.workgroup_count as u32, 1, 1);
    }

    //fn change_params() {}
}

/* This struct manages the GPU pipeline and creates the commands
 * needed to render the simulation and its parameters. */
struct ParticleRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl ParticleRenderer {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("draw.wgsl"));
        let pipeline_layout = device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let pipeline = device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: format,
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
        });

        Self { pipeline }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder, 
        storage: &mut ParticleStorage, 
        target: &wgpu::TextureView
    ) {
        let mut rpass = encoder 
            .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, storage.current_buffer().slice(..));
        rpass.draw(0..2, 0..storage.particle_count);
    }
}
