use crate::Result;
use ash::{khr, vk};
use std::ffi::CStr;

pub fn inspect(instance: &wgpu::Instance, adapter: &wgpu::Adapter) -> Result<()> {
    // Borrow the same Vulkan instance/physical device that wgpu selected.
    // We only query objects; never destroy borrowed handles or change display modes.
    unsafe {
        let hal = instance
            .as_hal::<wgpu::hal::api::Vulkan>()
            .ok_or("Not a Vulkan instance")?;
        let shared = hal.shared_instance();
        println!("instance_extensions={:?}", shared.extensions());
        if !shared.extensions().contains(&khr::display::NAME) {
            return Err("VK_KHR_display was not enabled".into());
        }
        let raw_adapter = adapter
            .as_hal::<wgpu::hal::api::Vulkan>()
            .ok_or("Not a Vulkan adapter")?;
        let physical = raw_adapter.raw_physical_device();
        let display = khr::display::Instance::new(shared.entry(), shared.raw_instance());
        let properties = display.get_physical_device_display_properties(physical)?;
        println!("display_count={}", properties.len());
        for (i, p) in properties.iter().enumerate() {
            let name = if p.display_name.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p.display_name)
                    .to_string_lossy()
                    .into_owned()
            };
            println!(
                "display={i} handle={:?} name={name:?} physical_resolution={:?} physical_dimensions={:?} transforms={:?}",
                p.display, p.physical_resolution, p.physical_dimensions, p.supported_transforms
            );
            let modes = display.get_display_mode_properties(physical, p.display)?;
            for (j, mode) in modes.iter().enumerate() {
                println!(
                    "display={i} mode={j} handle={:?} extent={:?} refresh_millihz={}",
                    mode.display_mode, mode.parameters.visible_region, mode.parameters.refresh_rate
                );
            }
        }
        let planes = display.get_physical_device_display_plane_properties(physical)?;
        println!("plane_count={}", planes.len());
        for (i, plane) in planes.iter().enumerate() {
            let supported = display.get_display_plane_supported_displays(physical, i as u32)?;
            println!(
                "plane={i} current_display={:?} stack_index={} supported={supported:?}",
                plane.current_display, plane.current_stack_index
            );
            for p in &properties {
                if !supported.contains(&p.display) {
                    continue;
                }
                for mode in display.get_display_mode_properties(physical, p.display)? {
                    let caps = display.get_display_plane_capabilities(
                        physical,
                        mode.display_mode,
                        i as u32,
                    )?;
                    println!(
                        "plane={i} mode={:?} alpha={:?} min_src={:?} max_src={:?} min_dst={:?} max_dst={:?}",
                        mode.display_mode,
                        caps.supported_alpha,
                        caps.min_src_extent,
                        caps.max_src_extent,
                        caps.min_dst_extent,
                        caps.max_dst_extent
                    );
                    // Test surface construction and capabilities, without a swapchain or presentation.
                    let create = vk::DisplaySurfaceCreateInfoKHR::default()
                        .display_mode(mode.display_mode)
                        .plane_index(i as u32)
                        .plane_stack_index(plane.current_stack_index)
                        .transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                        .global_alpha(1.0)
                        .alpha_mode(vk::DisplayPlaneAlphaFlagsKHR::OPAQUE)
                        .image_extent(mode.parameters.visible_region);
                    let surface = display.create_display_plane_surface(&create, None)?;
                    let surface_ext =
                        khr::surface::Instance::new(shared.entry(), shared.raw_instance());
                    let queried = (|| -> Result<()> {
                        println!("surface_created mode={:?}", mode.display_mode);
                        println!(
                            "surface_caps={:?}",
                            surface_ext
                                .get_physical_device_surface_capabilities(physical, surface)?
                        );
                        println!(
                            "surface_formats={:?}",
                            surface_ext.get_physical_device_surface_formats(physical, surface)?
                        );
                        println!(
                            "surface_present_modes={:?}",
                            surface_ext
                                .get_physical_device_surface_present_modes(physical, surface)?
                        );
                        println!(
                            "surface_queue0_support={}",
                            surface_ext
                                .get_physical_device_surface_support(physical, 0, surface)?
                        );
                        Ok(())
                    })();
                    surface_ext.destroy_surface(surface, None);
                    queried?;
                }
            }
        }
        if properties.is_empty() || planes.is_empty() {
            return Err("No display/plane reported".into());
        }
    }
    Ok(())
}

#[cfg(feature = "display-import")]
pub fn present(
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frames: u32,
) -> Result<()> {
    use std::{
        io::Write,
        sync::mpsc,
        time::{Duration, Instant},
    };
    println!("stage=display_surface");
    let (surface, extent) = unsafe {
        let hal = instance
            .as_hal::<wgpu::hal::api::Vulkan>()
            .ok_or("Not Vulkan")?;
        let shared = hal.shared_instance();
        if !shared.extensions().contains(&khr::display::NAME) {
            return Err("Missing VK_KHR_display".into());
        }
        let raw_adapter = adapter
            .as_hal::<wgpu::hal::api::Vulkan>()
            .ok_or("Not Vulkan")?;
        let physical = raw_adapter.raw_physical_device();
        let display = khr::display::Instance::new(shared.entry(), shared.raw_instance());
        let displays = display.get_physical_device_display_properties(physical)?;
        if displays.len() != 1 {
            return Err("Probe expects exactly one physical display".into());
        }
        let target = &displays[0];
        let modes = display.get_display_mode_properties(physical, target.display)?;
        let mode = modes
            .iter()
            .find(|m| m.parameters.visible_region == target.physical_resolution)
            .ok_or("No native-resolution mode")?;
        let planes = display.get_physical_device_display_plane_properties(physical)?;
        let mut selected = None;
        for (i, plane) in planes.iter().enumerate() {
            if display
                .get_display_plane_supported_displays(physical, i as u32)?
                .contains(&target.display)
            {
                let caps = display.get_display_plane_capabilities(
                    physical,
                    mode.display_mode,
                    i as u32,
                )?;
                if caps
                    .supported_alpha
                    .contains(vk::DisplayPlaneAlphaFlagsKHR::OPAQUE)
                {
                    selected = Some((i as u32, plane.current_stack_index));
                    break;
                }
            }
        }
        let (plane, stack) = selected.ok_or("No opaque compatible plane")?;
        let create = vk::DisplaySurfaceCreateInfoKHR::default()
            .display_mode(mode.display_mode)
            .plane_index(plane)
            .plane_stack_index(stack)
            .transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .global_alpha(1.0)
            .alpha_mode(vk::DisplayPlaneAlphaFlagsKHR::OPAQUE)
            .image_extent(mode.parameters.visible_region);
        let raw = display.create_display_plane_surface(&create, None)?;
        // Ownership of raw passes to HAL, then to wgpu. NativeSurface retains
        // the originating Vulkan instance; no raw handles are destroyed manually.
        let surface = instance
            .create_surface_from_hal::<wgpu::hal::api::Vulkan>(hal.import_display_surface(raw));
        (surface, mode.parameters.visible_region)
    };
    let caps = surface.get_capabilities(adapter);
    println!("wgpu_surface_caps={caps:?}");
    let mut config = surface
        .get_default_config(adapter, extent.width, extent.height)
        .ok_or("Surface incompatible")?;
    if !caps.formats.contains(&wgpu::TextureFormat::Bgra8Unorm)
        || !caps.usages.contains(wgpu::TextureUsages::COPY_SRC)
    {
        return Err("Probe requires BGRA8Unorm and surface COPY_SRC".into());
    }
    config.format = wgpu::TextureFormat::Bgra8Unorm;
    config.usage |= wgpu::TextureUsages::COPY_SRC;
    config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
    config.present_mode = wgpu::PresentMode::Fifo;
    config.desired_maximum_frame_latency = 1;
    println!("stage=configure config={config:?}");
    surface.configure(device, &config);
    let (pipeline, _) = crate::triangle_pipeline(device, config.format, crate::VertexMode::Buffer);
    let vertices = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("present-vertices"),
        size: 24,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let row_bytes = extent.width * 4;
    if row_bytes % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT != 0 {
        return Err("Unexpected unaligned display width".into());
    }
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("present-readback"),
        size: u64::from(row_bytes * extent.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let start = Instant::now();
    for frame in 0..frames {
        let offset = if frames == 1 {
            0.0
        } else {
            (frame as f32 / (frames - 1) as f32 * std::f32::consts::TAU).sin() * 0.15
        };
        let positions = [-0.7 + offset, -0.7_f32, 0.7 + offset, -0.7, offset, 0.7];
        let bytes: Vec<u8> = positions.into_iter().flat_map(f32::to_le_bytes).collect();
        queue.write_buffer(&vertices, 0, &bytes);
        let target = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            other => return Err(format!("Acquire failed: {other:?}").into()),
        };
        let view = target.texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present-triangle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
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
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.draw(0..3, 0..1);
        }
        if frame == frames - 1 {
            encoder.copy_texture_to_buffer(
                target.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(row_bytes),
                        rows_per_image: Some(extent.height),
                    },
                },
                wgpu::Extent3d {
                    width: extent.width,
                    height: extent.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let submission = queue.submit([encoder.finish()]);
        target.present();
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(15)),
        })?;
        if frame % 20 == 0 || frame == frames - 1 {
            println!("presented frame={frame}");
        }
    }
    let (tx, rx) = mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(15)),
    })?;
    rx.recv_timeout(Duration::from_secs(5))??;
    {
        let bytes = readback.slice(..).get_mapped_range();
        let mut rgba = Vec::with_capacity(bytes.len());
        for p in bytes.chunks_exact(4) {
            rgba.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
        crate::verify_extent(&rgba, 0.02, frames - 1, extent.width, extent.height)?;
        let mut file = std::fs::File::create("present.ppm")?;
        write!(file, "P6\n{} {}\n255\n", extent.width, extent.height)?;
        for p in rgba.chunks_exact(4) {
            file.write_all(&p[..3])?;
        }
    }
    readback.unmap();
    println!(
        "PRESENT_PASSED frames={frames} elapsed_ms={} size={}x{}",
        start.elapsed().as_millis(),
        extent.width,
        extent.height
    );
    Ok(())
}
