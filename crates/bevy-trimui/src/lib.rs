//! Native Bevy rendering helpers shared by the game and hardware probes.
//! Vulkan display ownership and the blit are confined to the Brick feature.

#[cfg(feature = "direct-display")]
mod display;
#[cfg(feature = "direct-display")]
pub use display::DisplayPresentPlugin;

use bevy::camera::RenderTarget;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Render, RenderApp,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            CachedPipelineState, Extent3d, PipelineCache, TextureDimension, TextureFormat,
            TextureUsages,
        },
        renderer::render_system,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    shader::ShaderCacheError,
};
use std::sync::{Arc, Mutex};

#[derive(Resource, Clone)]
pub struct CaptureImage(pub Handle<Image>);

/// Optional scene diagnostics extracted with the same frame as its transforms.
#[derive(Resource, Clone, ExtractResource)]
pub struct FrameMetadata(pub serde_json::Value);

#[derive(Resource, Clone, Default)]
pub struct RenderReadiness(Arc<Mutex<RenderReadinessState>>);

#[derive(Default)]
struct RenderReadinessState {
    ready_frames: u32,
    failed: Option<String>,
}

impl RenderReadiness {
    pub fn ready(&self) -> Result<bool, String> {
        let state = self.0.lock().unwrap();
        match &state.failed {
            Some(error) => Err(error.clone()),
            None => Ok(state.ready_frames >= 2),
        }
    }
}

fn record_render_readiness(cache: Res<PipelineCache>, readiness: Res<RenderReadiness>) {
    let mut state = readiness.0.lock().unwrap();
    let mut count = 0;
    let mut ready = true;
    for pipeline in cache.pipelines() {
        count += 1;
        match &pipeline.state {
            CachedPipelineState::Ok(_) => {}
            CachedPipelineState::Err(
                ShaderCacheError::ShaderNotLoaded(_)
                | ShaderCacheError::ShaderImportNotYetAvailable,
            ) => ready = false,
            CachedPipelineState::Err(error) => {
                state.failed = Some(error.to_string());
                ready = false;
            }
            _ => ready = false,
        }
    }
    if ready && count > 0 && cache.waiting_pipelines().next().is_none() {
        state.ready_frames += 1;
        if state.ready_frames == 2 {
            println!("stage=render_ready pipelines={count}");
        }
    } else {
        state.ready_frames = 0;
    }
}

/// Installs readiness tracking and an image target after DefaultPlugins.
pub fn add_offscreen_target(app: &mut App, width: u32, height: u32) -> Handle<Image> {
    let readiness = RenderReadiness::default();
    app.insert_resource(readiness.clone());
    app.insert_resource(FrameMetadata(serde_json::json!({})));
    app.add_plugins(ExtractResourcePlugin::<FrameMetadata>::default());
    app.sub_app_mut(RenderApp)
        .insert_resource(readiness)
        .add_systems(Render, record_render_readiness.after(render_system));
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.insert_resource(CaptureImage(target.clone()));
    app.insert_resource(NativeSnapshotTarget(RenderTarget::Image(
        target.clone().into(),
    )));
    target
}

#[derive(Resource)]
pub struct NativeSnapshotTarget(pub RenderTarget);

/// Screenshot completion is asynchronous; exit only after writing succeeds.
#[derive(Resource, Default)]
pub struct NativeSnapshot {
    request: Option<String>,
    pub pending: bool,
    pub completed: bool,
    pub failure: Option<String>,
}

impl NativeSnapshot {
    pub fn request(&mut self, path: String) {
        if !self.pending {
            self.request = Some(path);
            self.pending = true;
        }
    }
}

pub struct NativeSnapshotPlugin;

impl Plugin for NativeSnapshotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeSnapshot>()
            .add_systems(Update, capture_requested);
    }
}

fn capture_requested(
    mut commands: Commands,
    target: Res<NativeSnapshotTarget>,
    mut snapshot: ResMut<NativeSnapshot>,
    readiness: Res<RenderReadiness>,
) {
    match readiness.ready() {
        Ok(false) => return,
        Err(error) => {
            snapshot.failure = Some(error);
            return;
        }
        Ok(true) => {}
    }
    let Some(path) = snapshot.request.take() else {
        return;
    };
    commands.spawn(Screenshot(target.0.clone())).observe(
        move |captured: On<ScreenshotCaptured>,
              mut status: ResMut<NativeSnapshot>,
              metadata: Res<FrameMetadata>| {
            let result = captured
                .image
                .clone()
                .try_into_dynamic()
                .map_err(|error| format!("screenshot format: {error}"))
                .and_then(|image| {
                    let path = std::path::Path::new(&path);
                    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                        std::fs::create_dir_all(parent)
                            .map_err(|error| format!("screenshot directory: {error}"))?;
                    }
                    image
                        .to_rgb8()
                        .save(path)
                        .map_err(|error| format!("screenshot write: {error}"))?;
                    // Diagnostic world state at delivery; scripted QA waits
                    // for a stable state before requesting the GPU capture.
                    let bytes =
                        serde_json::to_vec_pretty(&metadata.0).map_err(|e| e.to_string())?;
                    std::fs::write(path.with_extension("json"), bytes).map_err(|e| e.to_string())
                });
            match result {
                Ok(()) => {
                    println!("SNAPSHOT: {path}");
                    status.completed = true;
                }
                Err(error) => status.failure = Some(error),
            }
        },
    );
}

pub fn spawn_hard_watchdog(seconds: f32) {
    std::thread::Builder::new()
        .name("hard-watchdog".into())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(seconds));
            eprintln!("WATCHDOG: no clean exit after {seconds:.0}s");
            std::process::exit(1);
        })
        .expect("spawn watchdog");
}

#[derive(Resource)]
struct Heartbeat(Arc<Mutex<std::time::Instant>>);

/// Interactive play has no fixed lifetime; terminate only if updates stop.
pub fn install_heartbeat_watchdog(app: &mut App, seconds: f32) {
    let heartbeat = Arc::new(Mutex::new(std::time::Instant::now()));
    app.insert_resource(Heartbeat(heartbeat.clone()));
    app.add_systems(Last, |heartbeat: Res<Heartbeat>| {
        *heartbeat.0.lock().unwrap() = std::time::Instant::now();
    });
    std::thread::Builder::new()
        .name("heartbeat-watchdog".into())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if heartbeat.lock().unwrap().elapsed().as_secs_f32() > seconds {
                    eprintln!("WATCHDOG: no game update for {seconds:.0}s");
                    std::process::exit(1);
                }
            }
        })
        .expect("spawn heartbeat watchdog");
}
