//! Headless rendering contract. Requires a software GPU adapter (Mesa on Linux).
use dragology_wgpu::{Renderer, scene::*};
use std::time::Duration;

struct Tile;
impl Diagram for Tile {
    type Id = u8;
    type State = ();
    fn layout(&self, _: &()) -> Vec<Item<u8>> {
        vec![Item {
            id: 0,
            quad: Quad {
                bounds: Rect::new(0., 0., 64., 64.),
                radius: 8.,
                color: [80, 160, 220, 255],
            },
        }]
    }
    fn drag(&self, _: &(), _: &u8) -> Option<DragSpec<'static, ()>> {
        None
    }
}

fn pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &Renderer,
    format: wgpu::TextureFormat,
) -> Vec<u8> {
    let size = wgpu::Extent3d {
        width: 64,
        height: 64,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("adapter test target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("adapter test readback"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        renderer.draw(&mut pass);
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap()
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receive
        .recv_timeout(Duration::from_secs(1))
        .unwrap()
        .unwrap();
    let bytes = readback.slice(..).get_mapped_range().unwrap().to_vec();
    readback.unmap();
    bytes
}

#[test]
fn rounded_canvas_preserves_color_clipping_and_clears_stale_draws() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        force_fallback_adapter: true,
        ..Default::default()
    }))
    .expect("install a Vulkan software adapter, e.g. mesa-vulkan-drivers on Linux");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let canvas = Canvas::new(Tile, (), Vec2::new(64., 64.));
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        let mut renderer = Renderer::new(&device, format, 1);
        renderer
            .prepare(
                &device,
                &queue,
                &canvas,
                Rect::new(16., 0., 80., 64.),
                [64, 64],
            )
            .unwrap();
        let bytes = pixels(&device, &queue, &renderer, format);
        let pixel = |x: usize, y: usize| &bytes[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
        for (actual, expected) in pixel(40, 32).iter().zip([80u8, 160, 220, 255]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "wrong color in {format:?}: {:?}",
                pixel(40, 32)
            );
        }
        assert_eq!(pixel(8, 32), &[0; 4], "outside the embedded viewport");
        assert_eq!(pixel(16, 0), &[0; 4], "outside the rounded corner");

        assert!(
            renderer
                .prepare(&device, &queue, &canvas, Rect::ZERO, [64, 64])
                .is_none()
        );
        assert!(
            pixels(&device, &queue, &renderer, format)
                .iter()
                .all(|byte| *byte == 0),
            "a collapsed viewport must not redraw the previous frame"
        );
    }
}
