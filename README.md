<h1 align="center"><img alt="niri" src="https://github.com/user-attachments/assets/07d05cd0-d5dc-4a28-9a35-51bae8f119a0"></h1>
<p align="center">A scrollable-tiling Wayland compositor — <strong>with HDR support</strong>.</p>
<p align="center">
    <em>This is <a href="https://github.com/ken-rikard/niri">ken-rikard/niri</a>, a feature fork of <a href="https://github.com/niri-wm/niri">niri-wm/niri</a> adding a full HDR rendering pipeline.</em>
</p>
<p align="center">
    <a href="https://github.com/ken-rikard/niri/blob/main/LICENSE"><img alt="GitHub License" src="https://img.shields.io/github/license/ken-rikard/niri"></a>
    <a href="https://github.com/ken-rikard/niri/compare/main...YaLTeR:niri:main"><img alt="Upstream diff" src="https://img.shields.io/badge/diff-upstream-blue"></a>
    <a href="https://matrix.to/#/#niri:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/matrix-%23niri-blue?logo=matrix"></a>
</p>

---

## HDR Support

This fork implements a **complete HDR rendering pipeline** for niri, supporting both PQ (ST 2084) and HLG transfer functions, SDR-to-HDR conversion, per-surface HDR passthrough, and ICC profile color correction.

### ✨ HDR Features

| Feature | Status |
|---------|--------|
| **PQ (Perceptual Quantizer) EOTF** — ST 2084 HDR10 rendering | ✅ Done |
| **HLG (Hybrid Log-Gamma)** — ARIB STD-B67 broadcast HDR | ✅ Done |
| **10-bit output** — `Xrgb2101010` framebuffer format with configurable bit depth | ✅ Done |
| **DRM metadata** — Sets `HDR_OUTPUT_METADATA` blob and `Colorspace=BT2020_RGB` | ✅ Done |
| **SDR→HDR conversion** — sRGB → linear light → BT.2020 → PQ encoding | ✅ Done |
| **SDR color intensity** — Gamut expansion (`sdr-color-intensity`, range 0.0–2.0) | ✅ Done |
| **Per-surface HDR passthrough** — Select apps output native HDR content | ✅ Done |
| **Per-element shader rendering** — Single-pass, no offscreen texture | ✅ Done |
| **Framebuffer fetch α-blending** — Correct blending in linear light (`GL_EXT_shader_framebuffer_fetch`) | ✅ Done |
| **Gamut mapping** — Desaturate, clip, and relative-perceptual modes | ✅ Done |
| **Dynamic metadata** — Configurable `max-cll`, `max-fall`, `min-luminance` | ✅ Done |
| **ICC profile color correction** — v2/v4 RGB display profiles via `icc-profile` option | ✅ Done |
| **EDID auto-configuration** — Reads display HDR capabilities from EDID on connect | ✅ Done |
| **IPC runtime control** — `niri msg output <name> hdr true --sdr-color-intensity 1.2` | ✅ Done |
| **Multiline HDR config** — Child-node syntax in `config.kdl` | ✅ Done |

### 🎮 HDR Configuration

Enable HDR in your `config.kdl` per output:

```kdl
output "HDMI-A-1" {
    hdr {
        enabled true
        max-luminance 800.0
        min-luminance 0.001
        max-cll 800.0
        max-fall 400.0
        sdr-brightness 0.5
        sdr-color-intensity 1.2
        passthrough-apps "mpv,steam"
        gamut-mapping "desaturate"
        transfer-function "pq"
    }
    icc-profile "/path/to/display.icc"
}
```

Or single-line syntax for quick configs:

```kdl
output "HDMI-A-1" hdr enabled=true max-luminance=800.0
```

If `hdr` is enabled without explicit luminance values, the compositor will attempt to read them from the display's EDID automatically. You can also use IPC to change settings at runtime:

```sh
niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.5 --gamut-mapping clip
```

### ⚙️ Architecture

The HDR pipeline uses a **single-pass per-element rendering** approach:

- Each `OutputRenderElements` is wrapped with `HdrWrappedElement`
- The wrapper calls `override_default_tex_program()` before drawing each element
- `GL_EXT_shader_framebuffer_fetch` decodes the PQ framebuffer and blends in linear light
- **No offscreen texture** — the DRM compositor handles damage tracking natively
- **Same performance as SDR rendering** — no extra FBO bind or GPU sync

Requires a [Smithay patch](patches/smithay-tex-program-override-stack.patch) for stackable shader overrides (needed for rounded corners + HDR).

### 🔧 Building

```sh
# Apply the required Smithay patch first
git apply patches/smithay-tex-program-override-stack.patch -p7
# or: patch -p1 < patches/smithay-tex-program-override-stack.patch

# Build as usual
cargo build --release
```

The patch modifies Smithay's GLES renderer to support stacking shader program overrides. See the [patch file](patches/smithay-tex-program-override-stack.patch) for details.

### 🧪 Memory Leak Note

The HDR shader system was observed to cause an OOM crash (~38 GB) when `shaders::init()` was called every frame without idempotency checks. This has been fixed — shader initialization is now guarded against recompilation. If you see memory growth, file an issue.

### 🐞 Known Issues

- **Cursor plane artifact**: A small transparent square may appear around the cursor on some GPU/driver combinations when HDR is active (`ALLOW_CURSOR_PLANE_SCANOUT` is disabled, but cursor `Kind::Cursor` may trigger other scanout paths).
- **Smithay patch dependency**: The stackable shader override patch must be applied to Smithay before building.

---

## About Niri

Windows are arranged in columns on an infinite strip going to the right.
Opening a new window never causes existing windows to resize.

Every monitor has its own separate window strip.
Windows can never "overflow" onto an adjacent monitor.

Workspaces are dynamic and arranged vertically.
Every monitor has an independent set of workspaces, and there's always one empty workspace present all the way down.

The workspace arrangement is preserved across disconnecting and connecting monitors where it makes sense.
When a monitor disconnects, its workspaces will move to another monitor, but upon reconnection they will move back to the original monitor.

## Features

- Built from the ground up for scrollable tiling
- [Dynamic workspaces](https://niri-wm.github.io/niri/Workspaces.html) like in GNOME
- An [Overview](https://github.com/user-attachments/assets/379a5d1f-acdb-4c11-b36c-e85fd91f0995) that zooms out workspaces and windows
- Built-in screenshot UI
- Monitor and window screencasting through xdg-desktop-portal-gnome
    - You can [block out](https://niri-wm.github.io/niri/Configuration%3A-Window-Rules.html#block-out-from) sensitive windows from screencasts
    - [Dynamic cast target](https://niri-wm.github.io/niri/Screencasting.html#dynamic-screencast-target) that can change what it shows on the go
- [Touchpad](https://github.com/niri-wm/niri/assets/1794388/946a910e-9bec-4cd1-a923-4a9421707515) and [mouse](https://github.com/niri-wm/niri/assets/1794388/8464e65d-4bf2-44fa-8c8e-5883355bd000) gestures
- Group windows into [tabs](https://niri-wm.github.io/niri/Tabs.html)
- Configurable layout: gaps, borders, struts, window sizes
- [Gradient borders](https://niri-wm.github.io/niri/Configuration%3A-Layout.html#gradients) with Oklab and Oklch support
- [Background blur](https://niri-wm.github.io/niri/Window-Effects.html) for windows and layer-shell surfaces
- [Animations](https://github.com/niri-wm/niri/assets/1794388/ce178da2-af9e-4c51-876f-8709c241d95e) with support for [custom shaders](https://github.com/niri-wm/niri/assets/1794388/27a238d6-0a22-4692-b794-30dc7a626fad)
- Live-reloading config
- Works with [screen readers](https://niri-wm.github.io/niri/Accessibility.html)

## Video Demo

https://github.com/niri-wm/niri/assets/1794388/bce834b0-f205-434e-a027-b373495f9729

Also check out these videos that showcase a lot of the niri functionality:

- [Niri Is My New Favorite Wayland Compositor](https://www.youtube.com/watch?v=DeYx2exm04M) by Brodie Robertson
- [How Is niri This Good? Live Demo + Config](https://www.youtube.com/watch?v=7XmD5UyyhZQ) by Nick Janetakis

## Status

Niri is stable for day-to-day use and does most things expected of a Wayland compositor.
Many people are daily-driving niri, and are happy to help in our [Matrix channel].

Give it a try!
Follow the instructions on the [Getting Started](https://niri-wm.github.io/niri/Getting-Started.html) page.
Grab a desktop shell like [DankMaterialShell] or [Noctalia] (or build a more traditional setup): niri by itself is not a complete desktop environment.
Also check out [awesome-niri], a list of niri-related links and projects.

Here are some points you may have questions about:

- **Multi-monitor**: yes, a core part of the design from the very start. Mixed DPI works.
- **Fractional scaling**: yes, plus all niri UI stays pixel-perfect.
- **NVIDIA**: seems to work fine.
- **Floating windows**: yes, starting from niri 25.01.
- **Input devices**: niri supports tablets, touchpads, and touchscreens.
You can map the tablet to a specific monitor, or use [OpenTabletDriver].
We have touchpad gestures, but no touchscreen gestures yet.
- **Wlr protocols**: yes, we have most of the important ones like layer-shell, gamma-control, screencopy.
You can check on [wayland.app](https://wayland.app) at the bottom of each protocol's page.
- **Performance**: while I run niri on beefy machines, I try to stay conscious of performance.
I've seen someone use it fine on an Eee PC 900 from 2008, of all things.
- **Xwayland**: [integrated](https://niri-wm.github.io/niri/Xwayland.html#using-xwayland-satellite) via xwayland-satellite starting from niri 25.08.

## Media

[niri: Making a Wayland compositor in Rust](https://youtu.be/Kmz8ODolnDg?list=PLRdS-n5seLRqrmWDQY4KDqtRMfIwU0U3T) · *December 2024*

My talk from the 2024 Moscow RustCon about niri, and how I do randomized property testing and profiling, and measure input latency.
The talk is in Russian, but I prepared full English subtitles that you can find in YouTube's subtitle language selector.

[An interview with Ivan, the developer behind Niri](https://www.trommelspeicher.de/podcast/special_the_developer_behind_niri) · *June 2025*

An interview by a German tech podcast Das Triumvirat (in English).
We talk about niri development and history, and my experience building and maintaining niri.

[A tour of the niri scrolling-tiling Wayland compositor](https://lwn.net/Articles/1025866/) · *July 2025*

An LWN article with a nice overview and introduction to niri.

## Contributing

If you'd like to help with niri, there are plenty of both coding- and non-coding-related ways to do so.
See [CONTRIBUTING.md](https://github.com/niri-wm/niri/blob/main/CONTRIBUTING.md) for an overview.

HDR-specific contributions are especially welcome! See [docs/hdr-implementation-plan.md](docs/hdr-implementation-plan.md) for the roadmap and [docs/hdr-testing-checklist.md](docs/hdr-testing-checklist.md) for testing procedures.

## Inspiration

Niri is heavily inspired by [PaperWM] which implements scrollable tiling on top of GNOME Shell.

One of the reasons that prompted me to try writing my own compositor is being able to properly separate the monitors.
Being a GNOME Shell extension, PaperWM has to work against Shell's global window coordinate space to prevent windows from overflowing.

## Tile Scrollably Elsewhere

Here are some other projects which implement a similar workflow:

- [PaperWM]: scrollable tiling on top of GNOME Shell.
- [karousel]: scrollable tiling on top of KDE.
- [scroll](https://github.com/dawsers/scroll) and [papersway]: scrollable tiling on top of sway/i3.
- Hyprland has a built-in [scrolling layout](https://wiki.hypr.land/Configuring/Layouts/Scrolling-Layout/).
- [Paneru] and [PaperWM.spoon]: scrollable tiling on top of macOS.

## Upstream

This is a feature fork of [niri-wm/niri](https://github.com/niri-wm/niri) by Ivan Molodetskikh.
All HDR-specific additions are being prepared for upstream merge.
The [feature/hdr-support](https://github.com/ken-rikard/niri/tree/feature/hdr-support) branch contains the PR-ready commit series.

## Contact

Our main communication channel is a Matrix chat, feel free to join and ask a question: https://matrix.to/#/#niri:matrix.org

We also have a community Discord server: https://discord.gg/vT8Sfjy7sx

[PaperWM]: https://github.com/paperwm/PaperWM
[waybar]: https://github.com/Alexays/Waybar
[fuzzel]: https://codeberg.org/dnkl/fuzzel
[awesome-niri]: https://github.com/niri-wm/awesome-niri
[karousel]: https://github.com/peterfajdiga/karousel
[papersway]: https://spwhitton.name/tech/code/papersway/
[Paneru]: https://github.com/karinushka/paneru
[PaperWM.spoon]: https://github.com/mogenson/PaperWM.spoon
[Matrix channel]: https://matrix.to/#/#niri:matrix.org
[OpenTabletDriver]: https://opentabletdriver.net/
[DankMaterialShell]: https://danklinux.com/
[Noctalia]: https://noctalia.dev/
