use std::fmt;
use std::sync::Arc;
use std::error::Error;

use winit::window::{Window, WindowId};
use winit::event::WindowEvent;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};

#[derive(Debug, Clone)]
enum FrameError {
    SkipFrame,
    Fatal
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for FrameError { 

}

const WORKGROUP_SIZE: u32 = 64;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
}

struct ParticleSimDescriptor<'a> {
    device: &'a wgpu::Device,
    surface_config: &'a wgpu::SurfaceConfiguration,
    num_particles: u32
}

struct ParticleSim {
    update_pipeline: wgpu::ComputePipeline,
    particle_bind_groups: Vec<wgpu::BindGroup>,
    particle_buffers: Vec<wgpu::Buffer>,
    workgroup_count: u32,
//    draw_pipeline: wgpu::RenderPipeline,
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
        let update_shader = device.create_shader_module(wgpu::include_wgsl!("update.wgsl"));

        let update_pipeline_layout = device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("update"),
                bind_group_layouts: &[Some(bind_group_layout)],
                immediate_size: 0
            });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("update_pipeline"), 
            layout: Some(&update_pipeline_layout),
            module: &update_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    fn create_particle_buffers(device: &wgpu::Device, num_particles: u32) -> Vec::<wgpu::Buffer> {
        let initial_particles = vec![
            Particle{ position: [0.0f32, 0.0f32], velocity: [0.0f32, 0.0f32] }; 
            num_particles as usize
        ];

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

    fn create_particle_bind_groups(device: &wgpu::Device, particle_buffers: &Vec::<wgpu::Buffer>) -> Vec::<wgpu::BindGroup> {
        let particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        particle_bind_groups
    }
}

impl ParticleSim {
    fn new(descriptor: &ParticleSimDescriptor) -> Self {
        let device = descriptor.device;
        let num_particles = descriptor.num_particles;

        let update_bind_group_layout = ParticleSim::create_update_bind_group_layout(device, num_particles);
        let update_pipeline = ParticleSim::create_update_pipeline(device, &update_bind_group_layout);
        let particle_buffers = ParticleSim::create_particle_buffers(device, num_particles);
        let particle_bind_groups = ParticleSim::create_particle_bind_groups(device, &particle_buffers);
//        let draw_shader = device.create_shader_module(wgpu::include_wgsl!("draw.wgsl"));
//
//        let draw_pipeline_layout =
//            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//                label: Some("draw"),
//                bind_group_layouts: &[],
//                immediate_size: 0,
//            });
//
//        let draw_pipeline = device
//            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//                label: Some("draw_pipeline"), 
//                layout: Some(&draw_pipeline_layout),
//                vertex: wgpu:: VertexState {
//                    module: &draw_shader,
//                    entry_point: Some("main_vs"),
//                    compilation_options: Default::default(),
//                    buffers: &[]
//                },
//                fragment: Some(wgpu::FragmentState {
//                    module: &draw_shader,
//                    entry_point: Some("main_fs"),
//                    compilation_options: Default::default(),
//                    targets: &[Some(surface_config.view_formats[0].into())],
//                }),
//                primitive: wgpu::PrimitiveState::default(),
//                depth_stencil: None,
//                multisample: wgpu::MultisampleState::default(),
//                multiview_mask: None,
//                cache: None,
//            });
        let workgroup_count = descriptor.num_particles.div_ceil(WORKGROUP_SIZE);

        Self {
            update_pipeline,
            particle_bind_groups,
            particle_buffers,
            workgroup_count
//            draw_pipeline
        }
    }


    fn update_particles(&mut self, encoder: &mut wgpu::CommandEncoder) {

    }

    fn draw_particles(&mut self, encoder: &mut wgpu::CommandEncoder) {
    }
}

struct State {
    window: Arc<Window>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    simulation: ParticleSim
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .unwrap();

        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .unwrap();
        
        let size = window.inner_size();

        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        let simulation = ParticleSim::new(&ParticleSimDescriptor{
            device: &device,
            surface_config: &surface_config,
            num_particles: 1000
        });

        let mut state = Self {
            window,
            instance,
            adapter,
            surface,
            surface_config,
            device,
            queue,
            size,
            simulation
        };

        state.configure_surface();
        state
    }

    fn configure_surface(&mut self) {
        self.surface_config = self.surface
            .get_default_config(&self.adapter, self.size.width, self.size.height)
            .unwrap();

        self.surface.configure(&self.device, &self.surface_config);
    }

    fn get_window(&self) -> &Window {
        return &self.window;
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.configure_surface();
    }

    fn acquire_surface_texture(&mut self) -> Result<wgpu::SurfaceTexture, FrameError> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return Err(FrameError::SkipFrame);
            },
            wgpu::CurrentSurfaceTexture::Occluded 
                | wgpu::CurrentSurfaceTexture::Timeout => return Err(FrameError::SkipFrame),

            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                return Err(FrameError::SkipFrame);
            }
            _ => return Err(FrameError::Fatal)
        };

        Ok(surface_texture)
    }

    fn frame(&mut self) {
        let surface_texture = match self.acquire_surface_texture() {
            Ok(texture) => texture,
            Err(FrameError::SkipFrame) => return,
            Err(FrameError::Fatal) => panic!("Fatal error while acquiring output texture")
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());

        self.simulation.update_particles(&mut encoder);
        self.simulation.draw_particles(&mut encoder);

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(surface_texture);
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap()
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                state.frame();
                state.get_window().request_redraw();
            },
            WindowEvent::Resized(size) => state.resize(size),
            _ => (),
        }
    }
}

fn main() {
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("Error while running application");
}
