# Device findings

This is a dated observation of one **TrimUI Brick Pro**, collected on 2026-10-08.
Do not silently apply it to the original Brick or another firmware revision.

| Item | Observed value |
| --- | --- |
| Firmware | 1.1.2 |
| System build | `tina.raymanfeng.20260828.020940`, Tina/OpenWrt 4.0.0 |
| Kernel | Linux 4.9.191, build #921, 2026-08-28 |
| GPU | PowerVR Rogue GE8300, vendor 0x1010, BVNC 22.102.54.38 |
| Kernel/firmware GPU DDK | 1.19@6345021 |
| Vulkan instance/device API | 1.3.225, one physical GPU |
| Queue family 0 | Graphics, compute, transfer; two queues |
| Maximum 2D texture dimension | 4096 |
| Panel used by the example | 1024 × 768 |
| DRM nodes | card0, controlD64, renderD128; driver name `pvr` |
| libc on disk | libc-2.33.so / ld-2.33.so |
| SDL runtime (historical comparison) | 2.30.8 |

The probe enumerated `VK_KHR_surface`, `VK_KHR_display`,
`VK_KHR_get_display_properties2` and `VK_EXT_headless_surface` instance extensions.
Subsequent drawing and presentation experiments, not this enumeration alone,
established the native rendering path.

`CONFIG_DRM=y` and a render node do not prove there is a standard usable KMS
display stack. The examined runtime directories had no X11/Xcb, Wayland, GBM or
libdrm runtime libraries. Use the implemented direct Vulkan display path rather
than assuming a desktop window surface is available. Khronos documents the
display/mode/plane API in [VK_KHR_display](https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_display.html).

## Why the published SDK was insufficient evidence

The local SDK used for the earlier experiment had SDL 2.26.1 and GPU build
1.11@5516664, and lacked the device's Vulkan libraries. Its EGL/GLES library
hashes differed from the running unit. Package metadata on the unit itself also
lagged: opkg reported libc 2.29 while the actual files were 2.33.

Use actual file hashes, ELF SONAME/NEEDED and runtime API queries. A library's
filename, an SDK label or a kernel configuration is not an execution test.

## Collect your own evidence

After configuring private SSH access:

```sh
uv run tools/device/snapshot.py
uv run tools/device/inspect_snapshot.py .device-snapshots/brick-pro-YOUR-TIMESTAMP
# Optional C-only enumeration; --sdk is a directory containing usr/include/vulkan.
uv run tools/device/probe.py \
  --snapshot .device-snapshots/brick-pro-YOUR-TIMESTAMP --sdk /path/to/sdk
```

Snapshots select runtime/include/module directories and explicit version/config
files; they exclude a full `/etc` dump, account/network configuration and SD saves.
They use tar streams without requiring rsync or installing a device service.
The manifest records archive and per-file hashes. Extracted absolute symlinks
are rebased into the snapshot, while the original archive preserves them.

A snapshot is neither a firmware backup nor a complete development sysroot.
Headers were sparse on the reference device. The optional C probe needs Vulkan
headers, but Rust wgpu uses ash bindings and dynamically loads Vulkan; the
minimal Bevy build needs no C Vulkan SDK. Supply `--sdk` explicitly if using
the C probe or SDK comparisons; the optional default is `.sdk` under the current
working directory. Keep dumps local and assess their separate redistribution terms.
