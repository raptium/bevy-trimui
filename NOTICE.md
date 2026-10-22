# Provenance and third-party materials

Original platform code, probes, scripts, documentation, Skill and procedural
launcher icon were extracted from an earlier private application. Their MIT
grant is in LICENSE. The grant applies to this kit, not to that application's assets.

Cargo downloads upstream dependencies independently. Bevy, wgpu, Naga, ash,
CPAL, ALSA bindings, serde_json, libc and other transitive dependencies retain
their own copyright and license terms. Consult their pinned crate metadata
and included license files. Modified wgpu and CPAL copies are generated under
`.scratch/` from actual crate archives and retain their upstream license files;
our license does not replace those files. Generated diffs are local compatibility
changes, not an upstream endorsement. When distributing binaries, include the
applicable dependency notices and licenses for the resolved dependency set.

No proprietary PowerVR/TrimUI libraries, firmware dump, paid art pack, game
assets, original repository history or private SSH configuration is included.
The snapshot utility is for obtaining evidence from a device you can access;
its output has independent redistribution terms and is ignored by Git.

TrimUI, Bevy and wgpu names identify the hardware and upstream projects. This
is a community experiment and platform starting point, not their official SDK.
