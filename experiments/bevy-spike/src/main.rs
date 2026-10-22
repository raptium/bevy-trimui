//! Minimal native Bevy 2D scene for the TrimUI Brick Pro.
//!
//! Offscreen mode (default): render to an image target, screenshot after
//! `--frames` frames, write `bevy-spike.ppm` and exit. This is the mode the
//! pixel verifier checks.
//!
//! Display mode (`--display`): additionally present each frame on the panel
//! through `VK_KHR_display` (see `display.rs`), run `--frames` frames, write
//! `display.ppm` from the presented surface and exit. `frames = 600` is about
//! ten seconds of visible output.
//!
//! The scene carries an asymmetric marker so a vertical flip anywhere in the
//! chain (image orientation, blit uv mapping) shows up in both PPMs.

use trimui_brick_render::{CaptureImage, DisplayPresentPlugin, FrameMetadata, RenderReadiness};
mod evdev;

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::mpsc::Receiver,
    time::Duration,
};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::RenderTarget,
    ecs::observer::On,
    prelude::*,
    render::view::{
        Msaa,
        screenshot::{Screenshot, ScreenshotCaptured},
    },
};

pub const WIDTH: u32 = 1024;
pub const HEIGHT: u32 = 768;
const SPRITE: f32 = 256.0;
const MARKER: f32 = 64.0;
const MARKER_Y: f32 = 200.0;
const DEFAULT_SCREENSHOT_FRAME: u32 = 20;
const DEFAULT_DISPLAY_FRAMES: u32 = 600;

#[derive(Component)]
struct MovingSprite;

#[derive(Resource, Clone)]
pub(crate) struct ScenePose {
    animated: bool,
    frame: u32,
    sprite_center_x: i32,
}

fn animate_scene(
    mut pose: ResMut<ScenePose>,
    mut sprites: Query<&mut Transform, With<MovingSprite>>,
) {
    if !pose.animated {
        return;
    }
    pose.frame += 1;
    let phase = ((pose.frame * 4) % 960) as i32;
    let offset = if phase <= 480 {
        phase - 240
    } else {
        720 - phase
    };
    pose.sprite_center_x = WIDTH as i32 / 2 + offset;
    for mut transform in &mut sprites {
        transform.translation.x = offset as f32;
    }
}

/// Decoded pad events, read directly because there is no working controller
/// layer on this rootfs. See `evdev.rs`.
#[derive(Resource)]
struct EvdevReader(std::sync::Mutex<Receiver<evdev::EvdevEvent>>);

impl EvdevReader {
    fn try_recv(&self) -> Option<evdev::EvdevEvent> {
        self.0.lock().ok()?.try_recv().ok()
    }
}

fn exit_on_gamepad(mut exit: MessageWriter<AppExit>, reader: Option<Res<EvdevReader>>) {
    let Some(reader) = reader else { return };
    while let Some(event) = reader.try_recv() {
        // BTN_MODE (Guide) or BTN_START, on press.
        if event.kind == evdev::EV_KEY && event.value == 1 && matches!(event.code, 0x13c | 0x13b) {
            println!("stage=gamepad_exit code=0x{:x}", event.code);
            exit.write(AppExit::Success);
        }
    }
}

/// Last-resort exit for the packaged app: the launcher must always get the
/// panel back, even if the render thread is stuck in a driver call and no
/// Bevy system can run anymore.
fn spawn_hard_watchdog(timeout: Duration) {
    let spawned = std::thread::Builder::new()
        .name("hard-watchdog".into())
        .spawn(move || {
            std::thread::sleep(timeout);
            eprintln!(
                "WATCHDOG: no clean exit after {:.0}s; forcing process exit",
                timeout.as_secs_f32()
            );
            std::process::exit(1);
        });
    if spawned.is_err() {
        eprintln!("WATCHDOG: thread could not be spawned");
    }
}

#[derive(Resource, Default)]
struct FrameCount {
    frame: u32,
    requested: bool,
}

#[derive(Resource)]
struct ScreenshotAt(u32);

fn main() -> AppExit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let display_mode = args.iter().any(|arg| arg == "--display");
    let animated = args.iter().any(|arg| arg == "--animate");
    let mut frames = if display_mode {
        DEFAULT_DISPLAY_FRAMES
    } else {
        DEFAULT_SCREENSHOT_FRAME
    };
    let mut frames_given = false;
    let mut seconds: Option<f32> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--display" | "--animate" => {}
            "--frames" => {
                index += 1;
                frames = args
                    .get(index)
                    .expect("--frames needs a count")
                    .parse()
                    .expect("--frames count must be a number");
                if !(1..=100_000).contains(&frames) {
                    eprintln!("--frames must be 1..100000");
                    std::process::exit(2);
                }
                frames_given = true;
            }
            "--seconds" => {
                index += 1;
                let value: f32 = args
                    .get(index)
                    .expect("--seconds needs a duration")
                    .parse()
                    .expect("--seconds duration must be a number");
                if value <= 0.0 {
                    eprintln!("--seconds must be positive");
                    std::process::exit(2);
                }
                seconds = Some(value);
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
        index += 1;
    }
    if seconds.is_some() && !display_mode {
        eprintln!("--seconds is only used with --display");
        std::process::exit(2);
    }
    if animated && !display_mode {
        eprintln!("--animate requires --display");
        std::process::exit(2);
    }

    println!(
        "stage=main bevy=0.19.1 wgpu=29.0.4 mode={}",
        if display_mode { "display" } else { "offscreen" }
    );
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16)));
    app.insert_resource(ScenePose {
        animated,
        frame: 0,
        sprite_center_x: WIDTH as i32 / 2,
    });
    app.add_systems(Update, animate_scene);
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)));
    let target = trimui_brick_render::add_offscreen_target(&mut app, WIDTH, HEIGHT);
    app.add_systems(Last, record_scene_metadata);
    app.add_systems(Startup, setup);

    if display_mode {
        // With --seconds, only the clock ends the run; otherwise --frames does.
        let frames = if frames_given || seconds.is_none() {
            frames
        } else {
            u32::MAX
        };

        // The launcher hands over the panel and expects it back. Three exits:
        // Guide/Start on the pad, the requested duration, and a process-level
        // watchdog for the case where the render thread is wedged.
        match evdev::start() {
            Some((path, receiver)) => {
                println!("stage=evdev node={}", path.display());
                app.insert_resource(EvdevReader(std::sync::Mutex::new(receiver)));
            }
            None => println!("stage=evdev_unavailable"),
        }
        app.add_systems(Update, exit_on_gamepad);
        let estimate = seconds.unwrap_or(frames as f32 / 30.0).max(1.0);
        spawn_hard_watchdog(Duration::from_secs_f32(
            estimate + (estimate * 0.25).max(15.0),
        ));
        println!(
            "stage=exit_options guide_or_start=quit seconds={:?} frames={}",
            seconds, frames
        );

        app.add_plugins(DisplayPresentPlugin::new(
            target,
            frames,
            seconds,
            Some("display.ppm".into()),
        ));
    } else {
        spawn_hard_watchdog(Duration::from_secs_f32(
            (frames as f32 / 30.0).max(30.0) + 15.0,
        ));
        app.insert_resource(ScreenshotAt(frames));
        app.init_resource::<FrameCount>();
        app.add_systems(Update, screenshot_after_warmup);
    }
    app.run()
}

fn record_scene_metadata(pose: Res<ScenePose>, mut metadata: ResMut<FrameMetadata>) {
    metadata.0 = serde_json::json!({"animated":pose.animated,"scene_frame":pose.frame,"sprite_center_x":pose.sprite_center_x});
}

fn setup(mut commands: Commands, capture: Res<CaptureImage>) {
    println!("stage=setup size={WIDTH}x{HEIGHT}");
    commands.spawn((
        Camera2d,
        RenderTarget::Image(capture.0.clone().into()),
        Msaa::Off,
    ));

    // 256x256 world units at the camera's default scaling: pixels.
    commands.spawn((
        MovingSprite,
        Sprite::from_color(Color::srgb(1.0, 0.5, 0.0), Vec2::splat(SPRITE)),
        Transform::default(),
    ));
    // Marker above the center; if a flip ever happens, it lands below.
    commands.spawn((
        Sprite::from_color(Color::srgb(0.1, 0.4, 1.0), Vec2::splat(MARKER)),
        Transform::from_xyz(0.0, MARKER_Y, 1.0),
    ));
}

fn screenshot_after_warmup(
    mut commands: Commands,
    mut frames: ResMut<FrameCount>,
    at: Res<ScreenshotAt>,
    capture: Res<CaptureImage>,
    readiness: Res<RenderReadiness>,
    mut exit: MessageWriter<AppExit>,
) {
    frames.frame += 1;
    let ready = match readiness.ready() {
        Ok(ready) => ready,
        Err(error) => {
            eprintln!("RENDER_FAILED: {error}");
            exit.write(AppExit::error());
            return;
        }
    };
    if frames.frame >= at.0 && !frames.requested && ready {
        frames.requested = true;
        println!("stage=screenshot_request frame={}", frames.frame);
        commands
            .spawn(Screenshot::image(capture.0.clone()))
            .observe(on_screenshot);
    }
}

fn on_screenshot(trigger: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>) {
    let image = &trigger.image;
    match image.data.as_deref() {
        Some(data) => {
            let width = image.texture_descriptor.size.width;
            let height = image.texture_descriptor.size.height;
            let mut file =
                BufWriter::new(File::create("bevy-spike.ppm").expect("create bevy-spike.ppm"));
            write!(file, "P6\n{width} {height}\n255\n").expect("write PPM header");
            for pixel in data.chunks_exact(4) {
                file.write_all(&pixel[..3]).expect("write PPM pixel");
            }
            file.flush().expect("flush PPM");
            println!(
                "stage=screenshot_saved path=bevy-spike.ppm size={width}x{height} bytes={}",
                data.len()
            );
        }
        None => {
            eprintln!("stage=screenshot_no_cpu_data");
            exit.write(AppExit::error());
            return;
        }
    }
    exit.write(AppExit::Success);
}
