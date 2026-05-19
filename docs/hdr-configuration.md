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

# Disable HDR
niri msg output HDMI-A-1 hdr false

# Check current HDR settings
niri msg outputs
```

**Note:** The IPC CLI uses snake_case (`--sdr-color-intensity`) which kebab-case
(`--sdr-color-intensity`). Both should work with clap.

---

## HDR Pipeline

The compositor applies the following pipeline when HDR is enabled:

```
sRGB input
  → linear conversion (BT.709 gamma → linear)
  → gamut expansion (SDR color intensity)
  → brightness scaling (SDR → nit space)
  → BT.2020 primaries conversion
  → PQ encoding (ST 2084)
  → DRM output (HDR metadata, BT.2020 colorspace, 10-bit)
```

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

### HDR content looks dim
- Verify `max-luminance` matches your display's actual peak brightness.
- Check your display's OSD for HDR settings.
- Try lowering `sdr-brightness` to allocate more headroom to HDR highlights.

### Display shows wrong colors or flickers
- Try a different `colorspace` (some displays prefer `bt2020`, others `display-p3`).
- Try a different `bit-depth` (some GPUs/drivers have issues with `16f`).
- Check your kernel and driver version (amdgpu needs a patched kernel for full colorspace support).

---

## Display Compatibility

| GPU | Status | Notes |
|-----|--------|-------|
| Intel (i915) | ✅ Good | Full colorspace and HDR metadata support |
| AMD (amdgpu) | ⚠️ Partial | Needs patched kernel for BT.2020 colorspace; HDR metadata works |
| NVIDIA (nvidia-drm) | ⚠️ Partial | HDR metadata works; colorspace support varies by driver version |