# HDR Debug History

## Current Status (2026-05-19)

When the pass-through shader is used (just `texture2D(tex, v_coords)`), the display shows grey instead of black — meaning the offscreen texture **does have content** and the HDR shader pipeline **does render**. The screen is grey because the offscreen texture was cleared to `TRANSPARENT` and the pass-through shader outputs that (which becomes grey on the DRM display).

## The Problem

The actual HDR conversion shader (sRGB→linear→BT.2020→PQ) produces black output. The pass-through shader produces grey. This means:

- ✅ Offscreen texture rendering works (content is correctly rendered to the FBO)
- ✅ The `HdrOutputRenderElement` draws (the shader is called)
- ✅ The texture sampler `tex` is correctly bound (pass-through shows content)
- ❌ The PQ/sRGB conversion in the shader produces black/zero output

## What Has Been Tried

### 1. Shader Compilation Fixes

**Problem:** Shader didn't compile for two reasons:
- `v_tex_coord` doesn't exist in smithay's vertex shader (it outputs `v_coords`)
- `uniform sampler2D niri_tex` doesn't bind — smithay binds `tex` via `glUniform1i(program.uniform_tex, 0)`

**Fix:** Changed to `varying vec2 v_coords` and `uniform sampler2D tex`.

**Status:** ✅ Working (confirmed by pass-through shader showing non-black output)

### 2. Shader Program Binding Approaches

**Approach A: `override_default_tex_program` (original)**

Called `frame.override_default_tex_program(program, uniforms)` then delegated `draw` to inner `TextureRenderElement`.

**Result:** Produced blue gradient color. Proved the shader program is valid and compiles. But sampling the texture returned black in the real shader.

**Approach B: Explicit `render_texture_from_to` with program parameter (current)**

Directly call `frame.render_texture_from_to(texture, src, dst, damage, &[], Transform::Normal, 1.0, Some(program), &uniforms)`.

**Result:** Pass-through shader shows grey content. Full HDR shader shows black. Proves the explicit program path works correctly.

### 3. Texture Binding Verification

Traced through smithay's `render_texture` in `src/backend/renderer/gles/mod.rs`. Confirmed:
- `gl.ActiveTexture(GL_TEXTURE0)` is called
- `gl.BindTexture(GL_TEXTURE_2D, tex.0.texture)` is called with the correct texture
- `gl.Uniform1i(program.uniform_tex, 0)` binds sampler to unit 0
- Custom uniforms (`u_sdr_brightness_nits`, `u_max_nits`, `u_sdr_color_intensity`) are set

**Status:** ✅ Texture binding is correct

### 4. Shader Math Investigation

The full HDR shader does:
1. `texture2D(tex, v_coords).rgb` — sample the sRGB texture
2. `srgb_to_linear()` — convert sRGB to linear
3. `expand_gamut()` — chroma scaling
4. `linear *= sdr_scale` — where `sdr_scale = sdr_brightness_nits / max_nits = 203/1000 = 0.203`
5. `srgb_to_bt2020()` — color space conversion
6. `pq_oetf()` — PQ encoding (ST 2084)

**Issue:** For white pixels (1.0, 1.0, 1.0):
- Linear = (1.0, 1.0, 1.0)
- After gamut expansion and scaling: (0.203, 0.203, 0.203)
- PQ(0.203) ≈ 0.62 on 0-1 scale → should be visible (mid-gray)

For the offscreen texture cleared to TRANSPARENT:
- Sampling transparent pixels gives some very small values
- After srgb_to_linear, these become even smaller
- After multiplying by 0.203 and PQ encoding, they could be near-zero

The grey screen with pass-through suggests the offscreen texture is mostly transparent/zero, so the HDR shader (which multiplies by 0.203 and PQ-encodes) produces black.

### 5. Offscreen Texture Clear Color

The offscreen texture is cleared with `Color32F::TRANSPARENT` before rendering elements. This means:
- Background = (0, 0, 0, 0) = transparent black
- Elements are rendered on top
- The pass-through shader shows grey because DRM displays transparent black as some non-zero color

## Current Hypothesis

The offscreen texture has **correct content** (confirmed by pass-through shader). The HDR shader's sRGB→linear→PQ conversion pipeline produces very dark results because:

1. The offscreen clear to `TRANSPARENT` (0, 0, 0, 0) leaves large areas at zero
2. The PQ encoding maps even small non-zero values to near-zero after the 203/1000 sdr_scale factor

**Need to verify:** What PQ value do actual window contents produce? The sRGB content from windows (like a white terminal background) should map to visible PQ values.

**Next debugging step:** Output a debug visualization that shows the raw sampled texture values alongside PQ-converted values to understand what the shader is actually computing.

## Key Files

- `src/render_helpers/shaders/hdr_output.frag` — HDR output shader (currently pass-through)
- `src/render_helpers/hdr_output.rs` — HdrOutputRenderElement (HDR program binding)
- `src/backend/tty.rs` — Offscreen rendering pipeline and HDR frame setup (lines ~2116-2270)
- `src/render_helpers/shaders/mod.rs` — Shader compilation (includes HDR shader at line ~148)
- `smithay/src/backend/renderer/gles/mod.rs` — GlesFrame::render_texture_from_to (line 2693), render_texture (line 2833)

## Build & Deploy

```bash
cargo build --release
# Binary at target/release/niri, symlinked to /usr/local/bin/niri-hdr
# SDDM session "Niri (HDR)" executes /usr/local/bin/niri-hdr --session
```

## Important Branch State

Current branch: `feature/hdr-sdr-intensity` at commit `18e433a1`
Base: `9db58ed5` (from `feature/hdr-support`)