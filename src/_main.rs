mod egui_tools;

use crate::egui_tools::EguiRenderer;
use egui::ahash::HashMapExt;
use egui_wgpu::ScreenDescriptor;
use image::GenericImageView;
use std::hash::Hash;
use std::sync::Arc;
use std::{default, thread};
use wgpu::naga::FastHashMap;
use wgpu::util::DeviceExt as _;
use wgpu::{Backends, InstanceDescriptor, TextureFormat};
use winit::dpi::{LogicalPosition, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::WindowLevel;

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let event_loop = EventLoop::new().unwrap();

    let builder = winit::window::WindowBuilder::new().with_transparent(true);
    let window = builder.build(&event_loop).unwrap();
    let window = Arc::new(window);

    let size = window.inner_size();

    let instance = wgpu::Instance::new(InstanceDescriptor::default());
    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: Default::default(),
                required_limits: Default::default(),
            },
            None,
        )
        .await
        .unwrap();
    let initial_width = 1360;
    let initial_height = 768;
    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let selected_format = TextureFormat::Bgra8UnormSrgb;
    let swapchain_format = swapchain_capabilities
        .formats
        .iter()
        .find(|d| **d == selected_format)
        .expect("failed to select proper surface texture format!");

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: *swapchain_format,
        width: initial_width,
        height: initial_height,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 0,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    surface.configure(&device, &config);

    let mut close_requested = false;
    let mut modifiers = ModifiersState::default();

    let mut scale_factor = 1.0;
    //

    let img = image::open("rats/house_rat.png").unwrap();
    let rgba = img.to_rgba8();
    let dimensions = img.dimensions();

    let texture_extent = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: Default::default(),
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: Default::default(),
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        texture_extent,
    );
    //

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    //

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Vertex {
        position: [f32; 3],
        tex_coords: [f32; 2],
    }

    impl Vertex {
        const ATTRIBS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

        fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &Self::ATTRIBS,
            }
        }
    }

    const VERTICES: &[Vertex] = &[
        Vertex {
            position: [-0.5, -0.5, 0.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            tex_coords: [1.0, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
            tex_coords: [0.0, 0.0],
        },
    ];

    const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });

    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Vertex Shader"),
        source: wgpu::ShaderSource::Glsl {
            shader: include_str!("vertex_shader.glsl").into(),
            stage: wgpu::naga::ShaderStage::Vertex,
            defines: FastHashMap::new(),
        },
    });

    let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fragment Shader"),
        source: wgpu::ShaderSource::Glsl {
            shader: include_str!("fragment_shader.glsl").into(),
            stage: wgpu::naga::ShaderStage::Fragment,
            defines: FastHashMap::new(),
        },
    });

    event_loop.run(move |event, control_flow| {
        match event {
            // Event::WindowEvent {
            //     ref event,
            //     window_id,
            // } if window_id == window.id() => {
            //     if !input.update(event) {
            //         match event {
            //             WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            //             WindowEvent::Resized(physical_size) => {
            //                 swap_chain = device.create_swap_chain(
            //                     &surface,
            //                     &wgpu::SwapChainDescriptor {
            //                         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            //                         format: swap_chain_format,
            //                         width: physical_size.width,
            //                         height: physical_size.height,
            //                         present_mode: wgpu::PresentMode::Fifo,
            //                     },
            //                 );
            //             }
            //             _ => {}
            //         }
            //     }
            // }
            // Event::RedrawRequested(_) => {
            //     let frame = swap_chain
            //         .get_current_frame()
            //         .expect("Failed to acquire next swap chain texture")
            //         .output;

            //     let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            //         label: Some("Render Encoder"),
            //     });

            //     let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            //         label: Some("Render Pass"),
            //         color_attachments: &[wgpu::RenderPassColorAttachment {
            //             view: &frame.view,
            //             resolve_target: None,
            //             ops: wgpu::Operations {
            //                 load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            //                 store: true,
            //             },
            //         }],
            //         depth_stencil_attachment: None,
            //     });

            //     render_pass.set_pipeline(&render_pipeline);
            //     render_pass.set_bind_group(0, &bind_group, &[]);
            //     render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            //     render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            //     render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);

            //     drop(render_pass);

            //     queue.submit(Some(encoder.finish()));

            //     // Handle frame updating logic here to cycle through sprites...
            // }
            // Event::MainEventsCleared => {
            //     // RedrawRequested will only be requested if the OS is requesting a redraw or we want to
            //     window.request_redraw();
            // }
            _ => {}
        }
    });
}
