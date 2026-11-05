use dragology_scene::{Canvas, Point, Pointer, Rect, Viewport};
use dragology_wgpu::Renderer;
use std::{error::Error, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

struct Gpu {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

impl Gpu {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("unsupported window surface")?;
        surface.configure(&device, &config);
        let renderer = Renderer::new(&device, config.format, 1);
        Ok(Self {
            window,
            instance,
            surface,
            device,
            queue,
            config,
            renderer,
        })
    }

    fn area(&self) -> Rect {
        let size = self.window.inner_size();
        Rect::new(0., 0., size.width.into(), size.height.into())
    }

    fn configure(&mut self) {
        let size = self.window.inner_size();
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn draw(&mut self, canvas: &Canvas<hanoi::Hanoi>) -> Result<(), Box<dyn Error>> {
        if self.area().is_zero_area() {
            return Ok(());
        }
        use wgpu::CurrentSurfaceTexture::*;
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            Success(frame) => (frame, false),
            Suboptimal(frame) => (frame, true),
            Lost => {
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.configure();
                self.window.request_redraw();
                return Ok(());
            }
            Outdated => {
                self.configure();
                self.window.request_redraw();
                return Ok(());
            }
            Timeout => {
                self.window.request_redraw();
                return Ok(());
            }
            Occluded => return Ok(()),
            Validation => return Err("surface validation failed".into()),
        };
        let target = frame.texture.create_view(&Default::default());
        self.renderer.prepare(
            &self.device,
            &self.queue,
            canvas,
            self.area(),
            [self.config.width, self.config.height],
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer.draw(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
        if suboptimal {
            self.configure();
        }
        Ok(())
    }
}

struct App {
    gpu: Option<Gpu>,
    canvas: Canvas<hanoi::Hanoi>,
    cursor: Option<Point>,
    last_frame: Instant,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(hanoi::TITLE)
            .with_inner_size(LogicalSize::new(hanoi::WIDTH, hanoi::HEIGHT));
        let window = Arc::new(el.create_window(attrs).expect("open Hanoi window"));
        self.gpu = Some(pollster::block_on(Gpu::new(window.clone())).expect("initialize wgpu"));
        window.request_redraw();
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(gpu) = &mut self.gpu else { return };
        let fit = Viewport::fit(self.canvas.size(), gpu.area());
        let input = match event {
            WindowEvent::CloseRequested => {
                el.exit();
                return;
            }
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    self.canvas.handle(Pointer::Cancel);
                }
                gpu.configure();
                gpu.window.request_redraw();
                return;
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let animating = self.canvas.advance(now - self.last_frame);
                self.last_frame = now;
                gpu.draw(&self.canvas).expect("draw Hanoi board");
                if animating {
                    gpu.window.request_redraw();
                }
                return;
            }
            WindowEvent::Occluded(false) => {
                gpu.window.request_redraw();
                return;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x, position.y);
                self.cursor = Some(point);
                fit.map(|v| Pointer::Move(v.to_scene(point)))
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                Some(Pointer::Cancel)
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.cursor.zip(fit).map(|(p, v)| {
                let point = v.to_scene(p);
                if state.is_pressed() {
                    Pointer::Press(point)
                } else {
                    Pointer::Release(point)
                }
            }),
            WindowEvent::Focused(false) => Some(Pointer::Cancel),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Character("r".into()) =>
            {
                self.canvas.set_model(Default::default());
                gpu.window.set_title(&format!(
                    "{} · {}",
                    hanoi::TITLE,
                    self.canvas.model().status()
                ));
                gpu.window.request_redraw();
                return;
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape) =>
            {
                Some(Pointer::Cancel)
            }
            _ => None,
        };
        if !self.canvas.is_animating() {
            self.last_frame = Instant::now();
        }
        if input.is_some_and(|event| self.canvas.handle(event)) {
            gpu.window.set_title(&format!(
                "{} · {}",
                hanoi::TITLE,
                self.canvas.model().status()
            ));
            gpu.window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    EventLoop::new()?.run_app(&mut App {
        gpu: None,
        canvas: hanoi::board(),
        cursor: None,
        last_frame: Instant::now(),
    })?;
    Ok(())
}
