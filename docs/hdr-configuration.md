# HDR Configuration Guide

This document covers the HDR configuration options available in niri. HDR allows
your display to show brighter highlights, deeper blacks, and more vivid colors
compared to standard dynamic range (SDR).

---

## Quick Start

1. **Ensure your display supports HDR.** Check with `niri msg outputs` — you
   should see an HDR-capable output.
2. **Enable HDR in your config** and set the correct max luminance for your display.
3. **Adjust SDR brightness** so SDR content looks right.
4. **Optionally tweak SDR color intensity** for more vibrant colors on HDR displays.

---

## Config Reference

HDR settings use **inline properties** on the `hdr` node in your output config:

```kdl
output "HDMI-A-1" {
    mode "3840x2160@120"
    scale 1.0

    // HDR is enabled as an inline property. Multiple properties can be combined.
    hdr enabled=true sdr-color-intensity=1.2 sdr-brightness=203
}
```

### Full HDR Configuration

```kdl
output "HDMI-A-1" {
    mode "3840x2160@120"
    scale 1.0
    variable-refresh-rate

    // All HDR settings go on the hdr line as properties.
    // Note: knuffel properties use kebab-case (sdr-color-intensity),
    // not snake_case (sdr_color_intensity).
    // Only the 'enabled' property is required; all others are optional.
    hdr \
        enabled=true \
        sdr-brightness=203 \
        sdr-color-intensity=1.2 \
        gamut-mapping="desaturate" \
        max-luminance=600 \
        colorspace=bt2020 \
        bit-depth=10
}
```

### Multiple Displays

Each output can have its own HDR configuration. SDR and HDR outputs can be mixed:

```kdl
// HDR-capable external monitor
output "HDMI-A-1" {
    mode "3840x2160@120"
    hdr enabled=true sdr-brightness=203 sdr-color-intensity=1.2
}

// Built-in laptop display (SDR only)
output "eDP-1" {
    mode "1920x1080@120"
    scale 2.0
    // No hdr line = SDR mode
}
```

---

## Settings Reference

**Important:** In the KDL config file, all multi-word property names use **kebab-case**
(with hyphens), not snake_case. The Rust field names use snake_case, but knuffel
automatically converts them.

| Config Property | Rust Field | Type | Range | Default | Description |
|----------------|------------|------|-------|---------|-------------|
| `enabled` | `enabled` | bool | `true`/`false` | `false` | Enable HDR on this output |
| `max-luminance` | `max_luminance` | float | — | EDID or 1000 | Peak brightness in nits |
| `min-luminance` | `min_luminance` | float | — | 0.005 | Minimum luminance in nits |
| `max-cll` | `max_cll` | float | — | max_luminance | Maximum content light level |
| `max-fall` | `max_fall` | float | — | max_luminance × 0.4 | Maximum frame-average light level |
| `sdr-brightness` | `sdr_brightness` | float | — | 203 | SDR white brightness in nits |
| `sdr-color-intensity` | `sdr_color_intensity` | float | 0.0–2.0 | 1.0 | Gamut expansion for SDR content |
| `gamut-mapping` | `gamut_mapping` | string | `"desaturate"`, `"clip"`, `"relative"` | `"desaturate"` | How to handle out-of-gamut colors after BT.2020 conversion |
| `passthrough-apps` | `passthrough_apps` | string | comma-separated | — | App-ids that output native HDR content (bypass SDR→HDR conversion) |
| `colorspace` | `colorspace` | string | `"srgb"`, `"display-p3"`, `"bt2020"` | `"bt2020"` | HDR color space |
| `bit-depth` | `bit_depth` | string | `"8"`, `"10"`, `"16f"` | `"10"` (auto) | Bit depth for HDR output |

---

## IPC Commands

You can change HDR settings at runtime using `niri msg`:

```bash
# Enable HDR (true/false is a positional argument, not --enabled)
niri msg output HDMI-A-1 hdr true

# Set SDR brightness (200 nits)
niri msg output HDMI-A-1 hdr true --sdr-brightness 203

# Set SDR color intensity (Phase 1)
niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.5

# Set gamut mapping mode
niri msg output HDMI-A-1 hdr true --gamut-mapping desaturate

# Set passthrough apps (comma-separated)
niri msg output HDMI-A-1 hdr true --passthrough-apps mpv,kodi

# Disable HDR
niri msg output HDMI-A-1 hdr false

# Check current HDR settings
niri msg outputs
```

**Note:** IPC commands perform **partial updates** — fields not specified retain their current values. For example, `niri msg output HDMI-A-1 hdr true --gamut-mapping clip` changes only the gamut mapping without affecting `sdr-color-intensity` or other settings.

---

## HDR Pipeline

The compositor applies the following pipeline when HDR is enabled. Each element
is individually processed by the HDR shader during the DRM compositor's single
render pass (no offscreen texture):

```
Per-element (via shader override):
  sRGB texture input (premultiplied alpha)
    → un-premultiply alpha
    → sRGB gamma decode (BT.709 EOTF)
    → gamut expansion (SDR color intensity)
    → sRGB → BT.2020 primaries conversion
    → gamut mapping (desaturate / clip / relative)
    → brightness scaling (× sdr_brightness nits)
    → PQ encoding (ST 2084 OETF)
    → linear-light alpha blending via framebuffer fetch
    → output to DRM framebuffer (Xrgb2101010, BT2020_RGB)
```

**Important:** Gamut mapping runs on normalized `[0,1]` values *before* scaling to nits. This prevents the mapping algorithms from seeing artificially large values and over-compressing colors.

### Alpha Blending

Semi-transparent elements (popups, overlays, window shadows) are correctly
composited using `GL_EXT_shader_framebuffer_fetch`. The shader reads the
current framebuffer value, decodes it from PQ to linear nits, blends with
the new element in linear light space, then re-encodes to PQ. This avoids
the severe darkening that would occur from blending PQ-encoded values directly.

If the extension is unavailable, a fallback path outputs premultiplied PQ values
and relies on GL blending (visually incorrect for semi-transparent overlays but
acceptable for fully opaque content).

### Performance

The per-element architecture matches SDR rendering performance:
- No offscreen texture or extra render pass
- DRM compositor handles damage tracking natively (only changed elements redraw)
- Single FBO (the swapchain buffer), no bind/unbind overhead
- Framebuffer fetch is essentially free (cache read on modern GPUs)

---

## Troubleshooting

### SDR content looks too dim
- Increase `sdr-brightness` (try 300-400 nits).
- This gives SDR white a higher luminance on the display.

### SDR content looks washed out
- Increase `sdr-color-intensity` (try 1.2–1.5).
- This expands the chroma of SDR content to utilize more of the wide gamut.

### SDR content looks oversaturated
- Decrease `sdr-color-intensity` (try 0.8–0.9).
- Some SDR content is already wide-gamut and may not need expansion.

### Wide-gamut SDR content has color banding or clipping
- Set `gamut-mapping="desaturate"` (default) to reduce saturation for out-of-gamut colors.
- Try `gamut-mapping="relative"` to preserve color relationships while compressing the gamut.
- Use `gamut-mapping="clip"` for simple clamping (may cause harsh transitions).

### Colors look wrong (e.g. red appears yellow)
- This indicates a color matrix issue. Ensure you're running the latest build.
- The sRGB→BT.2020 matrix must be in column-major order for GLSL.

### Semi-transparent overlays cause black screen
- Requires `GL_EXT_shader_framebuffer_fetch` GPU support.
- AMD (Mesa 26+), Intel, and most modern GPUs support this.
- If unsupported, fully opaque content renders correctly but semi-transparent
  overlays may appear too dark.

### HDR content looks dim
- Verify `max-luminance` matches your display's actual peak brightness.
- Check your display's OSD for HDR settings.
- Try lowering `sdr-brightness` to allocate more headroom to HDR highlights.
- If playing HDR video (mpv, etc.), add the app to `passthrough-apps` to bypass SDR→HDR conversion.

### Visible square artifact around cursor
- Known issue: the cursor hardware plane may bypass the HDR shader on some GPU/driver combinations.
- Workaround: the compositor disables cursor plane scanout in HDR mode, forcing the cursor through the shader path. If the artifact persists, it may be caused by direct element scanout — try `disable-direct-scanout=true` in the `[debug]` section.

### Display shows wrong colors or flickers
- Try a different `colorspace` (some displays prefer `bt2020`, others `display-p3`).
- Try a different `bit-depth` (some GPUs/drivers have issues with `16f`).
- Check your kernel and driver version (amdgpu needs a patched kernel for full colorspace support).

### Performance is poor with HDR enabled
- Ensure you're running a release build (`cargo build --release`).
- The per-element shader architecture should have no measurable overhead vs SDR.
- If performance is bad, check if `GL_EXT_shader_framebuffer_fetch` is falling
  back to the non-fetch path (unlikely to cause slowdowns but worth verifying).

---

## Display Compatibility

| GPU | Status | Notes |
|-----|--------|-------|
| AMD (amdgpu/RADV) | ✅ Tested | Mesa 26+, RDNA4 confirmed working. Needs `GL_EXT_shader_framebuffer_fetch` (supported via Zink/radeonsi) |
| Intel (i915) | ✅ Expected | Full colorspace and HDR metadata support, framebuffer fetch available |
| NVIDIA (nvidia-drm) | ⚠️ Untested | HDR metadata should work; framebuffer fetch support depends on driver version |

### Requirements
- GPU with 10-bit output support
- Display that accepts HDR10 (PQ + BT.2020)
- `GL_EXT_shader_framebuffer_fetch` for correct semi-transparent overlay rendering
- DRM atomic modesetting with `Colorspace` and `HDR_OUTPUT_METADATA` properties