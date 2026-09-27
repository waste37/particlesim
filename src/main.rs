mod particlesim;

use std::fmt;
use std::time;
use std::sync::Arc;
use std::error::Error;

use winit::window::{Window, WindowId};
use winit::event::{WindowEvent, MouseButton, ElementState};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

use particlesim::{ParticleSystem, ParticleSimulationInput};

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

struct FrameContext {
    encoder: wgpu::CommandEncoder,
    surface_texture: wgpu::SurfaceTexture,
    target: wgpu::TextureView,
    delta_time: f32
}

struct State {
    window: Arc<Window>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    frame_time: time::Instant,
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
        
        let mut state = Self {
            window,
            instance,
            adapter,
            surface,
            device,
            queue,
            size,
            frame_time: time::Instant::now(),
        };

        state.configure_surface();
        state
    }

    fn configure_surface(&mut self) {
        let surface_config = self.surface
            .get_default_config(&self.adapter, self.size.width, self.size.height)
            .unwrap();

        self.surface.configure(&self.device, &surface_config);
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

    fn begin_frame(&mut self) -> Result<FrameContext, FrameError> {
        let surface_texture = match self.acquire_surface_texture() {
            Ok(texture) => texture,
            Err(other) => return Err(other)
        };

        let delta_time = self.frame_time.elapsed().as_secs_f32();
        self.frame_time = time::Instant::now();

        let encoder = self.device.create_command_encoder(&Default::default());
        let target = surface_texture.texture.create_view(&Default::default());

        return Ok(FrameContext {encoder, surface_texture, target, delta_time});
    }

    fn end_frame(&self, ctx: FrameContext) {
        self.queue.submit(Some(ctx.encoder.finish()));
        self.queue.present(ctx.surface_texture);
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
    simulation: Option<ParticleSystem>,
    input: ParticleSimulationInput,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap()
        );

        let state = pollster::block_on(State::new(window.clone()));

        let render_format = state.surface.get_configuration().as_ref().unwrap().format;
        let simulation = ParticleSystem::new(&state.device, render_format, 10000);

        self.state = Some(state);
        self.simulation = Some(simulation);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        let simulation = self.simulation.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {

                let mut ctx = match state.begin_frame() {
                    Ok(ctx) => ctx,
                    Err(FrameError::SkipFrame) => return,
                    Err(FrameError::Fatal) => panic!("Fatal error while acquiring output texture")    
                };

                simulation.update(&mut ctx.encoder, &self.input, ctx.delta_time);
                simulation.render(&mut ctx.encoder, &ctx.target);

                state.end_frame(ctx);
                state.get_window().request_redraw();
            },
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::MouseInput { 
                button: MouseButton::Left, state, .. 
            } => match state { 
                ElementState::Pressed => self.input.mouse_down = true,
                ElementState::Released => self.input.mouse_down = false,
            },
            WindowEvent::CursorMoved { position, .. } => { 
                self.input.mouse_position = [position.x as f32, position.y as f32];
            }
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
