use std::sync::Arc;

use winit::window::{Window, WindowId};
use winit::event::WindowEvent;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

struct State {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration
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
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        surface.configure(&device, &config);
        return Self {
            window,
            instance,
            surface,
            device,
            queue,
            config
        }
    }

    fn get_window(&self) -> &Window {
        return &self.window;
    }

    fn render(&mut self) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.surface.configure(&self.device, &self.config);
                return;
            },
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => panic!("error in obtaining surface texture")
        };

        let view = surface_texture.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear screen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        drop(render_pass);

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(surface_texture);
    }
}

#[derive(Default)]
struct App {
    state: Option<State>
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
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            },
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("Error while running application");
}
