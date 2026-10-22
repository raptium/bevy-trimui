//! Present the offscreen Bevy frame on the Brick's panel through VK_KHR_display.
//!
//! There is no window system on the device. The camera renders into an image
//! target; after Bevy's render graph has run and submitted (the `render_system`
//! system does both), this module blits that image onto a wgpu surface created
//! directly from a `VK_KHR_display` plane and presents it. The raw surface
//! import goes through the wgpu-hal / wgpu-core / wgpu display-import patch
//! prepared by `tools/device/prepare_wgpu_display.py`.
//!
//! Everything wgpu-owned lives in the render world; the main world only sees
//! the flags it needs to exit. The game and diagnostic probe share this code.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ash::{khr, vk};
use bevy::{
    app::AppExit,
    ecs::message::MessageWriter,
    prelude::*,
    render::{
        Render, RenderApp,
        render_asset::RenderAssets,
        renderer::{RenderAdapter, RenderDevice, RenderInstance, RenderQueue, render_system},
        texture::GpuImage,
    },
};

use crate::CaptureImage;

/// Adds the display presentation path. Only constructed when `--display` is
/// passed; the default offscreen screenshot mode is unaffected.
pub struct DisplayPresentPlugin {
    image: Handle<Image>,
    frames: u32,
    seconds: Option<f32>,
    readback_path: Option<String>,
    flags: SharedFlags,
}

impl DisplayPresentPlugin {
    pub fn new(
        image: Handle<Image>,
        frames: u32,
        seconds: Option<f32>,
        readback_path: Option<String>,
    ) -> Self {
        Self {
            image,
            frames,
            seconds,
            readback_path,
            flags: SharedFlags::default(),
        }
    }
}

impl Plugin for DisplayPresentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.flags.clone());
        app.add_systems(Update, exit_on_display_end);

        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(RenderDisplay {
            target: None,
            flags: self.flags.clone(),
        });
        render_app.insert_resource(DisplayConfig {
            frames: self.frames,
            seconds: self.seconds,
            readback_path: self.readback_path.clone(),
        });
        render_app.insert_resource(CaptureImage(self.image.clone()));
        render_app.add_systems(
            Render,
            (
                init_display_state,
                present_frame
                    .after(render_system)
                    .after(crate::record_render_readiness),
            )
                .chain(),
        );
    }
}

/// Flags the main world may read. Never contains wgpu types.
#[derive(Resource, Clone, Default)]
struct SharedFlags(Arc<Mutex<Flags>>);

#[derive(Default)]
struct Flags {
    failed: Option<String>,
    finished: bool,
    frame: u32,
}

#[derive(Resource)]
struct DisplayConfig {
    frames: u32,
    seconds: Option<f32>,
    readback_path: Option<String>,
}

#[derive(Resource, Default)]
struct RenderDisplay {
    target: Option<Result<DisplayTarget, String>>,
    flags: SharedFlags,
}

struct DisplayTarget {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    readback: Option<Readback>,
    started: std::time::Instant,
    snapshots: Vec<Snapshot>,
}

struct Snapshot {
    path: String,
    pixels: Vec<u8>,
    pose: crate::FrameMetadata,
    presented_frame: u32,
}

struct Readback {
    buffer: wgpu::Buffer,
    path: String,
}

fn exit_on_display_end(flags: Res<SharedFlags>, mut exit: MessageWriter<AppExit>) {
    let flags = flags.0.lock().unwrap();
    if let Some(message) = &flags.failed {
        eprintln!("DISPLAY_FAILED: {message}");
        exit.write(AppExit::error());
    } else if flags.finished {
        exit.write(AppExit::Success);
    }
}

fn init_display_state(
    mut display: ResMut<RenderDisplay>,
    config: Res<DisplayConfig>,
    instance: Res<RenderInstance>,
    adapter: Res<RenderAdapter>,
    device: Res<RenderDevice>,
) {
    if display.target.is_some() {
        return;
    }
    // Bevy's RenderDevice has its own wrapper methods; go through raw wgpu.
    let render_instance: &RenderInstance = &instance;
    let render_adapter: &RenderAdapter = &adapter;
    let instance: &wgpu::Instance = &*render_instance.0;
    let adapter: &wgpu::Adapter = &*render_adapter.0;
    let device: &wgpu::Device = device.wgpu_device();
    let result = create_display_target(instance, adapter, device, &config);
    match &result {
        Ok(target) => println!(
            "stage=display_ready size={}x{} format={:?} frames={}",
            target.config.width, target.config.height, target.config.format, config.frames
        ),
        Err(message) => eprintln!("stage=display_init_failed error={message}"),
    }
    let failed = result.as_ref().err().cloned();
    display.target = Some(result);
    let mut flags = display.flags.0.lock().unwrap();
    if let Some(message) = failed {
        flags.failed = Some(message);
    }
}

fn present_frame(
    mut display: ResMut<RenderDisplay>,
    config: Res<DisplayConfig>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
    capture: Res<CaptureImage>,
    readiness: Res<crate::RenderReadiness>,
    pose: Res<crate::FrameMetadata>,
) {
    let RenderDisplay { target, flags } = &mut *display;
    let Some(Ok(target)) = target else { return };
    {
        let state = flags.0.lock().unwrap();
        if state.finished || state.failed.is_some() {
            return;
        }
    }
    {
        let state = readiness.0.lock().unwrap();
        if let Some(error) = &state.failed {
            flags.0.lock().unwrap().failed = Some(error.clone());
            return;
        }
        if state.ready_frames < 2 {
            return;
        }
    }
    let Some(gpu_image) = images.get(&capture.0) else {
        return;
    };
    // Raw wgpu handles: Bevy's wrappers have their own method sets/types.
    let device: &wgpu::Device = device.wgpu_device();
    let render_queue: &RenderQueue = &queue;
    let queue: &wgpu::Queue = &*render_queue.0;

    let acquired = match target.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(texture)
        | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            target.surface.configure(device, &target.config);
            return;
        }
        other => {
            println!("stage=display_acquire_skipped reason={other:?}");
            return;
        }
    };

    let view = acquired.texture.create_view(&Default::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("display-blit"),
        layout: &target.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&gpu_image.texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&target.sampler),
            },
        ],
    });

    let mut flags_lock = flags.0.lock().unwrap();
    let deadline_passed = config
        .seconds
        .is_some_and(|seconds| target.started.elapsed().as_secs_f32() >= seconds);
    let final_frame = flags_lock.frame + 1 >= config.frames.max(1) || deadline_passed;
    let sample_frame = pose.0["animated"].as_bool().unwrap_or(false)
        && matches!(flags_lock.frame + 1, 1 | 60 | 120 | 180 | 240);
    if final_frame {
        println!(
            "stage=display_finishing frame={} elapsed_ms={}",
            flags_lock.frame + 1,
            target.started.elapsed().as_millis()
        );
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("display-blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        pass.set_pipeline(&target.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
    if final_frame || sample_frame {
        if let Some(readback) = &target.readback {
            encoder.copy_texture_to_buffer(
                acquired.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback.buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(target.config.width * 4),
                        rows_per_image: Some(target.config.height),
                    },
                },
                wgpu::Extent3d {
                    width: target.config.width,
                    height: target.config.height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    queue.submit([encoder.finish()]);
    acquired.present();

    flags_lock.frame += 1;
    if flags_lock.frame % 60 == 0 {
        println!("stage=display_presented frame={}", flags_lock.frame);
    }
    if final_frame || sample_frame {
        if let Some(readback) = &target.readback {
            let pixels = match read_display_pixels(device, readback) {
                Ok(pixels) => pixels,
                Err(error) => {
                    flags_lock.failed = Some(error);
                    return;
                }
            };
            let path = if final_frame {
                readback.path.clone()
            } else {
                format!("display-{:04}.ppm", flags_lock.frame)
            };
            println!(
                "stage=display_sample frame={} scene_frame={} sprite_center_x={}",
                flags_lock.frame, pose.0["scene_frame"], pose.0["sprite_center_x"]
            );
            target.snapshots.push(Snapshot {
                path,
                pixels,
                pose: pose.clone(),
                presented_frame: flags_lock.frame,
            });
        }
    }
    if final_frame {
        for snapshot in &target.snapshots {
            if let Err(error) = write_display_snapshot(snapshot, &target.config) {
                flags_lock.failed = Some(error);
                return;
            }
        }
        flags_lock.finished = true;
    }
}

fn create_display_target(
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    config: &DisplayConfig,
) -> Result<DisplayTarget, String> {
    let (surface, surface_config) = unsafe { create_display_surface(instance, adapter, device)? };
    let width = surface_config.width;
    let height = surface_config.height;
    if width * 4 % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT != 0 && config.readback_path.is_some() {
        return Err(format!("{width}px rows are not copy-aligned"));
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("display-blit"),
        source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("display-blit-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("display-blit"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("display-blit"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("display-blit"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let readback = config.readback_path.as_ref().map(|path| Readback {
        buffer: device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("display-readback"),
            size: u64::from(width) * u64::from(height) * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }),
        path: path.clone(),
    });
    Ok(DisplayTarget {
        surface,
        config: surface_config,
        pipeline,
        bind_group_layout,
        sampler,
        readback,
        started: std::time::Instant::now(),
        snapshots: Vec::new(),
    })
}

/// Borrow the renderer's Vulkan instance to create a `VK_KHR_display` surface
/// and hand ownership to wgpu through the display-import patch. Mirrors the
/// probe's `display::present` surface setup.
unsafe fn create_display_surface(
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
) -> Result<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration), String> {
    // SAFETY: this borrows the renderer's own Vulkan instance and physical
    // device to create a display surface that wgpu then takes ownership of.
    // No borrowed handle is destroyed here.
    unsafe {
        let hal = instance
            .as_hal::<wgpu::hal::api::Vulkan>()
            .ok_or("renderer instance is not Vulkan")?;
        let shared = hal.shared_instance();
        if !shared.extensions().contains(&khr::display::NAME) {
            return Err("VK_KHR_display was not enabled on this instance".into());
        }
        let raw_adapter = adapter
            .as_hal::<wgpu::hal::api::Vulkan>()
            .ok_or("renderer adapter is not Vulkan")?;
        let physical = raw_adapter.raw_physical_device();
        let display = khr::display::Instance::new(shared.entry(), shared.raw_instance());

        let displays = display
            .get_physical_device_display_properties(physical)
            .map_err(|error| format!("display properties: {error}"))?;
        let target = displays
            .iter()
            .find(|display| display.physical_resolution.width > 0)
            .ok_or("no physical display reported")?;
        let modes = display
            .get_display_mode_properties(physical, target.display)
            .map_err(|error| format!("display modes: {error}"))?;
        let mode = modes
            .iter()
            .find(|mode| mode.parameters.visible_region == target.physical_resolution)
            .or_else(|| modes.first())
            .ok_or("no display mode reported")?;

        let planes = display
            .get_physical_device_display_plane_properties(physical)
            .map_err(|error| format!("display planes: {error}"))?;
        let mut selected = None;
        for (index, plane) in planes.iter().enumerate() {
            let supported = display
                .get_display_plane_supported_displays(physical, index as u32)
                .map_err(|error| format!("plane {index} displays: {error}"))?;
            if !supported.contains(&target.display) {
                continue;
            }
            let caps = display
                .get_display_plane_capabilities(physical, mode.display_mode, index as u32)
                .map_err(|error| format!("plane {index} caps: {error}"))?;
            if caps
                .supported_alpha
                .contains(vk::DisplayPlaneAlphaFlagsKHR::OPAQUE)
            {
                selected = Some((index as u32, plane.current_stack_index));
                break;
            }
        }
        let (plane, stack) = selected.ok_or("no opaque-compatible display plane")?;

        let create = vk::DisplaySurfaceCreateInfoKHR::default()
            .display_mode(mode.display_mode)
            .plane_index(plane)
            .plane_stack_index(stack)
            .transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .global_alpha(1.0)
            .alpha_mode(vk::DisplayPlaneAlphaFlagsKHR::OPAQUE)
            .image_extent(mode.parameters.visible_region);
        let raw = display
            .create_display_plane_surface(&create, None)
            .map_err(|error| format!("create display surface: {error}"))?;

        // Ownership of `raw` passes to the HAL, then to wgpu. No manual destroy.
        let surface = instance
            .create_surface_from_hal::<wgpu::hal::api::Vulkan>(hal.import_display_surface(raw));

        let caps = surface.get_capabilities(adapter);
        let extent = mode.parameters.visible_region;
        let mut config = surface
            .get_default_config(adapter, extent.width, extent.height)
            .ok_or("display surface is not compatible with the render adapter")?;
        config.format = [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
        ]
        .into_iter()
        .find(|format| caps.formats.contains(format))
        .ok_or_else(|| format!("no BGRA8 display format in {:?}", caps.formats))?;
        config.usage |= wgpu::TextureUsages::COPY_SRC;
        config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency = 1;
        surface.configure(device, &config);
        Ok((surface, config))
    }
}

fn read_display_pixels(device: &wgpu::Device, readback: &Readback) -> Result<Vec<u8>, String> {
    let slice = readback.buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let poll = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    });
    if let Err(error) = poll {
        return Err(format!("display readback poll: {error}"));
    }
    match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => {}
        other => {
            return Err(format!("display readback map: {other:?}"));
        }
    }
    let pixels = slice.get_mapped_range().to_vec();
    readback.buffer.unmap();
    Ok(pixels)
}

fn write_display_snapshot(
    snapshot: &Snapshot,
    config: &wgpu::SurfaceConfiguration,
) -> Result<(), String> {
    let width = config.width;
    let height = config.height;
    let mut data = format!("P6\n{width} {height}\n255\n").into_bytes();
    data.reserve(width as usize * height as usize * 3);
    // BGRA8 on the display; PPM wants RGB. Write files only at exit, so SD
    // card I/O does not stop the animation at each sampled frame.
    for pixel in snapshot.pixels.chunks_exact(4) {
        data.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    std::fs::write(&snapshot.path, data)
        .map_err(|error| format!("display readback file: {error}"))?;
    let mut metadata = snapshot.pose.0.clone();
    metadata["presented_frame"] = snapshot.presented_frame.into();
    let metadata =
        serde_json::to_vec(&metadata).map_err(|error| format!("display metadata: {error}"))?;
    std::fs::write(
        std::path::Path::new(&snapshot.path).with_extension("json"),
        metadata,
    )
    .map_err(|error| format!("display readback metadata: {error}"))?;
    println!(
        "stage=display_snapshot_saved path={} size={width}x{height}",
        snapshot.path
    );
    Ok(())
}
