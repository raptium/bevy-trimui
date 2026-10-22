#!/usr/bin/env python3
"""Add an isolated raw Vulkan surface import path across wgpu layers."""
from pathlib import Path
import difflib
import hashlib
import json
import shutil
import subprocess
import sys
import argparse
import os

ROOT = Path(__file__).resolve().parents[2]
ap = argparse.ArgumentParser(description=__doc__)
ap.add_argument("--version", default="29.0.4")
ap.add_argument("--cargo-home", type=Path, default=Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))))
ap.add_argument("--vendor-dir", type=Path, default=ROOT / ".scratch/wgpu-probe/vendor")
args = ap.parse_args()
if args.version not in ("29.0.4", "30.0.1"):
    raise SystemExit("Surface import API has only been verified for wgpu 29.0.4 and 30.0.1")
vendor = args.vendor_dir.resolve()
subprocess.run([sys.executable, str(ROOT / "tools/device/prepare_wgpu_hal.py"),
                "--version", args.version, "--cargo-home", str(args.cargo_home),
                "--vendor-dir", str(vendor)], check=True)
for name in ("wgpu-core", "wgpu"):
    matches = sorted((args.cargo_home / "registry/src").glob(f"*/{name}-{args.version}"))
    if len(matches) != 1:
        raise SystemExit(f"Expected one cached {name} {args.version}")
    shutil.copytree(matches[0], vendor / name, dirs_exist_ok=True)

changes = [
    ("wgpu-hal", "src/vulkan/instance.rs", "    pub(super) fn create_surface_from_vk_surface_khr(", """    /// Import an owned surface created with this exact Vulkan instance.
    ///
    /// # Safety
    /// The surface must be valid, unused, and owned by the caller. Ownership
    /// transfers to wgpu-hal; the caller must not destroy it afterward.
    pub unsafe fn import_display_surface(&self, surface: vk::SurfaceKHR) -> super::Surface {
        self.create_surface_from_vk_surface_khr(surface)
    }

    pub(super) fn create_surface_from_vk_surface_khr("""),
    ("wgpu-core", "src/instance.rs", "impl Global {\n    /// Creates a new surface targeting the given display/window handles.", """impl Global {
    /// Import a HAL surface from this Global's matching backend instance.
    ///
    /// # Safety
    /// The surface must belong to this instance and ownership transfers here.
    pub unsafe fn instance_create_surface_from_hal<A: hal::Api>(
        &self, raw: A::Surface,
    ) -> SurfaceId {
        let raw: Box<dyn hal::DynSurface> = Box::new(raw);
        let surface = Surface {
            presentation: Mutex::new(rank::SURFACE_PRESENTATION, None),
            surface_per_backend: core::iter::once((A::VARIANT, raw)).collect(),
        };
        self.surfaces.prepare(None).assign(Arc::new(surface))
    }

    /// Creates a new surface targeting the given display/window handles."""),
    ("wgpu", "src/backend/wgpu_core.rs", "impl ContextWgpuCore {\n", """impl ContextWgpuCore {
    pub unsafe fn surface_from_hal<A: hal::Api>(&self, surface: A::Surface) -> dispatch::DispatchSurface {
        let id = unsafe { self.0.instance_create_surface_from_hal::<A>(surface) };
        CoreSurface {
            context: self.clone(), id,
            configured_device: Mutex::default(), error_sink: Mutex::default(),
        }.into()
    }

"""),
    ("wgpu", "src/api/instance.rs", "/// Interop with wgpu-hal.\n#[cfg(wgpu_core)]\nimpl Instance {\n", """/// Interop with wgpu-hal.
#[cfg(wgpu_core)]
impl Instance {
    /// Import an owned HAL surface created from this same instance.
    ///
    /// # Safety
    /// The surface must belong to this instance, be usable, and transfer its
    /// ownership here. Its native display must outlive the returned Surface.
    pub unsafe fn create_surface_from_hal<A: hal::Api>(&self, surface: A::Surface) -> Surface<'static> {
        let inner = unsafe { self.inner.as_core().surface_from_hal::<A>(surface) };
        Surface { inner, config: Mutex::new(None), _handle_source: None }
    }

"""),
]
if args.version == "30.0.1":
    crate, relative, old, new = changes[0]
    changes[0] = (crate, relative, old, new.replace(
        "self.create_surface_from_vk_surface_khr(surface)",
        "self.create_surface_from_vk_surface_khr(surface, None)"))
records = []
patches = {}
for crate, relative, old, new in changes:
    path = vendor / crate / relative
    original = path.read_text()
    if original.count(old) != 1:
        raise SystemExit(f"Expected unique patch anchor in {path}")
    patched = original.replace(old, new)
    path.write_text(patched)
    patches.setdefault(crate, []).append("".join(difflib.unified_diff(
        original.splitlines(True), patched.splitlines(True), fromfile=f"a/{relative}", tofile=f"b/{relative}")))
    records.append({"crate": crate, "file": relative,
                    "original_sha256": hashlib.sha256(original.encode()).hexdigest(),
                    "patched_sha256": hashlib.sha256(patched.encode()).hexdigest()})
for crate, patch in patches.items():
    (vendor / f"{crate}-display-import.patch").write_text("".join(patch))
(vendor / "display-provenance.json").write_text(json.dumps(records, indent=2) + "\n")
print("Prepared raw surface import in wgpu-hal, wgpu-core and wgpu")
