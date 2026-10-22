use std::{
    error::Error,
    future::Future,
    io::Write,
    sync::{Arc, mpsc},
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
mod display;
const SIZE: u32 = 256;
const ROW_BYTES: u32 = SIZE * 4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum VertexMode {
    Buffer,
    Array,
    Branch,
    ArrayStore,
    ArrayConst,
    Select,
    VecDynamic,
    MatrixDynamic,
    UniformArray,
    StorageArray,
    ArrayVec3,
    ArrayVec4,
    ArrayF32,
    ArrayInit,
    ArrayAssign,
}

// This probe needs only one future at a time, with no async runtime.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWake(std::thread::Thread);
    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    install_crash_trace();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    if let Err(error) = run() {
        eprintln!("PROBE_FAILED: {error}");
        std::process::exit(1);
    }
}

// Diagnostic-only native backtrace for crashes inside the vendor .so files.
// A remote watchdog bounds execution if the unwinder itself cannot complete.
#[cfg(target_os = "linux")]
fn install_crash_trace() {
    unsafe extern "C" {
        fn signal(sig: i32, handler: unsafe extern "C" fn(i32)) -> usize;
        fn backtrace(buffer: *mut *mut std::ffi::c_void, size: i32) -> i32;
        fn backtrace_symbols_fd(buffer: *const *mut std::ffi::c_void, size: i32, fd: i32);
        fn _exit(code: i32) -> !;
    }
    unsafe extern "C" fn handler(sig: i32) {
        let mut frames = [std::ptr::null_mut(); 32];
        unsafe {
            let count = backtrace(frames.as_mut_ptr(), frames.len() as i32);
            backtrace_symbols_fd(frames.as_ptr(), count, 2);
            _exit(128 + sig);
        }
    }
    unsafe {
        signal(11, handler);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let display_only = args.iter().any(|s| s == "--display-info");
    let present = args.iter().any(|s| s == "--present");
    let mut vertex_mode = VertexMode::Buffer;
    let mut frames = 3_u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--display-info" | "--present" => {}
            "--vertex-array" => {
                vertex_mode = VertexMode::Array;
            }
            "--vertex-branch" => {
                vertex_mode = VertexMode::Branch;
            }
            "--vertex-array-store" => {
                vertex_mode = VertexMode::ArrayStore;
            }
            "--vertex-array-const" => {
                vertex_mode = VertexMode::ArrayConst;
            }
            "--vertex-select" => {
                vertex_mode = VertexMode::Select;
            }
            "--vertex-vec" => {
                vertex_mode = VertexMode::VecDynamic;
            }
            "--vertex-matrix" => {
                vertex_mode = VertexMode::MatrixDynamic;
            }
            "--vertex-uniform-array" => {
                vertex_mode = VertexMode::UniformArray;
            }
            "--vertex-storage-array" => {
                vertex_mode = VertexMode::StorageArray;
            }
            "--vertex-array-vec3" => {
                vertex_mode = VertexMode::ArrayVec3;
            }
            "--vertex-array-vec4" => {
                vertex_mode = VertexMode::ArrayVec4;
            }
            "--vertex-array-f32" => {
                vertex_mode = VertexMode::ArrayF32;
            }
            "--vertex-array-init" => {
                vertex_mode = VertexMode::ArrayInit;
            }
            "--vertex-array-assign" => {
                vertex_mode = VertexMode::ArrayAssign;
            }
            "--frames" => {
                i += 1;
                frames = args.get(i).ok_or("Missing --frames count")?.parse()?;
                if !(1..=1000).contains(&frames) {
                    return Err("--frames must be 1..1000".into());
                }
            }
            _ => return Err(format!("Unknown argument: {}", args[i]).into()),
        }
        i += 1;
    }
    let start = Instant::now();
    println!("stage=instance wgpu=29.0.4 backend=Vulkan");
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::VULKAN;
    // Keep wgpu validation, and its normal driver compliance checks enabled.
    descriptor.flags = wgpu::InstanceFlags::VALIDATION;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        force_fallback_adapter: false,
        power_preference: wgpu::PowerPreference::LowPower,
    }))?;
    let info = adapter.get_info();
    println!("adapter={info:?}");
    println!("features={:?}", adapter.features());
    println!("limits={:?}", adapter.limits());
    println!("downlevel={:?}", adapter.get_downlevel_capabilities());
    if info.backend != wgpu::Backend::Vulkan || info.device_type == wgpu::DeviceType::Cpu {
        return Err("Expected a hardware Vulkan adapter".into());
    }
    if display_only {
        display::inspect(&instance, &adapter)?;
        println!("DISPLAY_QUERY_PASSED");
        return Ok(());
    }
    let required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
    println!(
        "stage=device required_limits=wgpu_defaults_with_adapter_resolution required_features=empty"
    );
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("boat-wgpu-probe"),
        required_limits,
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        ..Default::default()
    }))?;
    if present {
        #[cfg(feature = "display-import")]
        {
            return display::present(&instance, &adapter, &device, &queue, frames);
        }
        #[cfg(not(feature = "display-import"))]
        {
            return Err("Rebuild with build_wgpu.py --display for --present".into());
        }
    }
    println!("stage=pipeline");
    println!("vertex_mode={vertex_mode:?}");
    let (pipeline, bind_group) =
        triangle_pipeline(&device, wgpu::TextureFormat::Rgba8Unorm, vertex_mode);
    let vertices = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("triangle-vertices"),
        size: 24,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let positions = [-0.7_f32, -0.7, 0.7, -0.7, 0.0, 0.7];
    let bytes: Vec<u8> = positions.into_iter().flat_map(f32::to_le_bytes).collect();
    queue.write_buffer(&vertices, 0, &bytes);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pixel-readback"),
        size: u64::from(ROW_BYTES * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    // Changing the background across submissions checks that readback is fresh.
    for frame in 0..frames {
        println!("stage=submit frame={frame}");
        let red = 0.02 + f64::from(frame % 3) * 0.05;
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("offscreen-triangle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: red,
                            g: 0.04,
                            b: 0.08,
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
            pass.set_pipeline(&pipeline);
            if let Some(bind_group) = &bind_group {
                pass.set_bind_group(0, bind_group, &[]);
            }
            if vertex_mode == VertexMode::Buffer {
                pass.set_vertex_buffer(0, vertices.slice(..));
            }
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ROW_BYTES),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        println!("stage=finish_encoder frame={frame}");
        let command = encoder.finish();
        println!("stage=queue_submit frame={frame}");
        let submission = queue.submit([command]);
        println!("stage=map frame={frame}");
        let (tx, rx) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        println!("stage=poll frame={frame}");
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(15)),
        })?;
        rx.recv_timeout(Duration::from_secs(5))??;
        {
            let pixels = buffer.slice(..).get_mapped_range();
            if frame == 0 || frame == frames - 1 {
                let mut file = std::fs::File::create(format!("triangle-{frame}.ppm"))?;
                write!(file, "P6\n{SIZE} {SIZE}\n255\n")?;
                for pixel in pixels.chunks_exact(4) {
                    file.write_all(&pixel[..3])?;
                }
            }
            verify(&pixels, red, frame)?;
        }
        println!("stage=unmap frame={frame}");
        buffer.unmap();
    }
    println!(
        "PROBE_PASSED frames={frames} elapsed_ms={}",
        start.elapsed().as_millis()
    );
    Ok(())
}

fn verify(pixels: &[u8], red: f64, frame: u32) -> Result<()> {
    verify_extent(pixels, red, frame, SIZE, SIZE)
}

fn verify_extent(pixels: &[u8], red: f64, frame: u32, width: u32, height: u32) -> Result<()> {
    let mut checked = 0;
    let mut mismatches = 0;
    let background = [(red * 255.0).round() as u8, 10, 20, 255];
    for y in 0..height {
        for x in 0..width {
            let nx = (f64::from(x) + 0.5) * 2.0 / f64::from(width) - 1.0;
            let ny = 1.0 - (f64::from(y) + 0.5) * 2.0 / f64::from(height);
            let edges = [ny + 0.7, 0.7 - ny - 2.0 * nx, 0.7 - ny + 2.0 * nx];
            // Ignore a small band around edges to tolerate rasterization rounding.
            if edges.iter().any(|v| v.abs() < 0.025) {
                continue;
            }
            let inside = edges.iter().all(|v| *v > 0.0);
            let expected = if inside {
                [255, 64, 0, 255]
            } else {
                background
            };
            let offset = ((y * width + x) * 4) as usize;
            if pixels[offset..offset + 4]
                .iter()
                .zip(expected)
                .any(|(a, b)| a.abs_diff(b) > 1)
            {
                mismatches += 1;
            }
            checked += 1;
        }
    }
    println!(
        "readback frame={frame} checked={checked} mismatches={mismatches} center={:?} corner={:?}",
        &pixels[((height / 2 * width + width / 2) * 4) as usize..][..4],
        &pixels[..4]
    );
    if mismatches != 0 || checked < width * height * 9 / 10 {
        return Err(format!("Pixel comparison failed: {mismatches}/{checked}").into());
    }
    Ok(())
}

fn triangle_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    mode: VertexMode,
) -> (wgpu::RenderPipeline, Option<wgpu::BindGroup>) {
    let source = match mode {
        VertexMode::Buffer => include_str!("triangle.wgsl"),
        VertexMode::Array => include_str!("triangle-array.wgsl"),
        VertexMode::Branch => include_str!("triangle-branch.wgsl"),
        VertexMode::ArrayStore => include_str!("array-store.wgsl"),
        VertexMode::ArrayConst => include_str!("array-const.wgsl"),
        VertexMode::Select => include_str!("select.wgsl"),
        VertexMode::VecDynamic => include_str!("vec-dynamic.wgsl"),
        VertexMode::MatrixDynamic => include_str!("matrix-dynamic.wgsl"),
        VertexMode::UniformArray => include_str!("uniform-array.wgsl"),
        VertexMode::StorageArray => include_str!("storage-array.wgsl"),
        VertexMode::ArrayVec3 => include_str!("array-vec3.wgsl"),
        VertexMode::ArrayVec4 => include_str!("array-vec4.wgsl"),
        VertexMode::ArrayF32 => include_str!("array-f32.wgsl"),
        VertexMode::ArrayInit => include_str!("array-init.wgsl"),
        VertexMode::ArrayAssign => include_str!("array-assign.wgsl"),
    };
    let vertex_layouts = [wgpu::VertexBufferLayout {
        array_stride: 8,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        }],
    }];
    // The two buffer-backed modes keep the same triangle in GPU memory instead
    // of in the shader, to tell function-local indexing apart from memory.
    let mut bind_group = None;
    let mut bind_group_layout = None;
    if matches!(mode, VertexMode::UniformArray | VertexMode::StorageArray) {
        let uniform = mode == VertexMode::UniformArray;
        // The uniform variant uses vec4 elements (16-byte stride); the storage
        // variant uses a runtime-sized vec2 array, the shape Bevy's Mesh2d
        // shader indexes (`mesh[instance_index]`).
        let data: Vec<u8> = if uniform {
            [
                [-0.7_f32, -0.7, 0.0, 0.0],
                [0.7, -0.7, 0.0, 0.0],
                [0.0, 0.7, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ]
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect()
        } else {
            [[-0.7_f32, -0.7], [0.7, -0.7], [0.0, 0.7], [0.0, 0.0]]
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect()
        };
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("triangle-array-data"),
            size: data.len() as u64,
            usage: if uniform {
                wgpu::BufferUsages::UNIFORM
            } else {
                wgpu::BufferUsages::STORAGE
            },
            mapped_at_creation: true,
        });
        buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(&data);
        buffer.unmap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("triangle-array-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: if uniform {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage { read_only: true }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("triangle-array"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        bind_group = Some(group);
        bind_group_layout = Some(layout);
    }
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("triangle"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline_layout = bind_group_layout.as_ref().map(|layout| {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("triangle"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        })
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("triangle"),
        layout: pipeline_layout.as_ref(),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: if mode == VertexMode::Buffer {
                &vertex_layouts
            } else {
                &[]
            },
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    (pipeline, bind_group)
}
