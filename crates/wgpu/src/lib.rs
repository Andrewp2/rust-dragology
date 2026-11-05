//! Draw a Dragology canvas into an existing wgpu color pass. The host retains
//! ownership of the window, surface, device, render loop and pointer input.
use bytemuck::{Pod, Zeroable};
pub use dragology_scene as scene;
use scene::{Canvas, Diagram, Rect, Viewport};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    center_clip: [f32; 2],
    half_clip: [f32; 2],
    half_pixels: [f32; 2],
    radius: f32,
    color: [f32; 4],
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    srgb: bool,
    instances: Vec<Instance>,
    buffer: Option<wgpu::Buffer>,
    clip: [u32; 4],
}

/// Convert an sRGB channel to the linear value expected by an sRGB attachment.
fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

impl Renderer {
    /// Match the color format and sample count of the host's render pass.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("quad.wgsl"));
        const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32, 4 => Float32x4
        ];
        let instances = wgpu::VertexBufferLayout {
            array_stride: size_of::<Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBUTES,
        };
        let target = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Dragology rounded rectangles"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(instances)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(target)],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            srgb: format.is_srgb(),
            instances: Vec::new(),
            buffer: None,
            clip: [0; 4],
        }
    }

    /// Upload one canvas. `area` and `target` are physical pixels. The returned
    /// fit maps host pointer coordinates back to this canvas with `to_scene`.
    /// Use a separate renderer per canvas when drawing several in one submission.
    pub fn prepare<D: Diagram>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &Canvas<D>,
        area: Rect,
        target: [u32; 2],
    ) -> Option<Viewport> {
        self.instances.clear();
        let [w, h] = target;
        if w == 0 || h == 0 {
            return None;
        }
        let view = Viewport::fit(canvas.size(), area)?;
        let clip = area.intersect(Rect::new(0., 0., w.into(), h.into()));
        if clip.is_zero_area() {
            return None;
        }
        let clip = clip.expand();
        self.clip = [
            clip.x0 as u32,
            clip.y0 as u32,
            clip.width() as u32,
            clip.height() as u32,
        ];
        for quad in canvas.quads() {
            let q = view.quad(quad);
            let b = q.bounds;
            if b.intersect(clip).is_zero_area() || q.color[3] == 0 {
                continue;
            }
            let mut color = q.color.map(|n| f32::from(n) / 255.);
            if self.srgb {
                for c in &mut color[..3] {
                    *c = linear(*c);
                }
            }
            let center = b.center();
            self.instances.push(Instance {
                center_clip: [
                    (2. * center.x / f64::from(w) - 1.) as f32,
                    (1. - 2. * center.y / f64::from(h)) as f32,
                ],
                half_clip: [
                    (b.width() / f64::from(w)) as f32,
                    (b.height() / f64::from(h)) as f32,
                ],
                half_pixels: [b.width() as f32 * 0.5, b.height() as f32 * 0.5],
                radius: q.radius as f32,
                color,
            });
        }
        if !self.instances.is_empty() {
            let data = bytemuck::cast_slice(&self.instances);
            if self
                .buffer
                .as_ref()
                .is_none_or(|b| b.size() < data.len() as u64)
            {
                self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Dragology instances"),
                    size: data.len().next_power_of_two() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            queue.write_buffer(self.buffer.as_ref().unwrap(), 0, data);
        }
        Some(view)
    }

    /// Draw the prepared canvas. Assumes a full-target viewport and sets the
    /// pipeline, scissor and vertex buffer. Rebind those for subsequent host draws.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.instances.is_empty() {
            return;
        }
        let [x, y, w, h] = self.clip;
        pass.set_scissor_rect(x, y, w, h);
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.buffer.as_ref().unwrap().slice(..));
        pass.draw(0..4, 0..self.instances.len() as u32);
    }
}
