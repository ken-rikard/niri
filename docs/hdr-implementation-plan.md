# HDR Implementation Plan for Niri

## Current Status

Core HDR pipeline is functional:
- ✅ Static HDR metadata via `HDR_OUTPUT_METADATA` DRM property
- ✅ Colorspace set to `BT2020_RGB`
- ✅ 10-bit output format selection (`Xrgb2101010`)
- ✅ SDR→HDR rendering pipeline with PQ EOTF (ST 2084)
- ✅ HDR config parsing from `config.kdl`
- ✅ Gamma control disabled when HDR active
- ✅ Color management protocol structure (data types exist)
- ✅ **Per-element HDR shader rendering** (single-pass, no offscreen texture)
- ✅ **Framebuffer fetch for correct alpha blending** (`GL_EXT_shader_framebuffer_fetch`)
- ✅ **SDR color intensity (gamut expansion)** — configurable 0.0–2.0
- ✅ **sRGB→BT.2020 color matrix** (corrected column-major order)
- ✅ **IPC support** for runtime HDR enable/disable and parameter changes
- ✅ **ICC profile color correction** — Per-display ICC v2/v4 profile loading and matrix application in HDR shader
- ✅ **EDID auto-configuration** — Auto-detect display HDR capabilities from EDID and use as defaults
- ✅ **Multiline HDR config** — `hdr { enabled max-luminance 730.0 ... }` child-node syntax
- ✅ **Passthrough shader framebuffer fetch** — Correct alpha blending for HDR-native content

### Architecture

The HDR rendering uses a **per-element shader override** architecture:
- Each `OutputRenderElements` is wrapped in `HdrWrappedElement`
- The wrapper calls `override_default_tex_program()` before drawing each element
- **Stackable overrides** — When `ClippedSurfaceRenderElement` (rounded corners) needs its own program, it appends uniforms to the existing HDR override instead of replacing it (requires Smithay patch)
- The DRM compositor handles damage tracking natively (no extra offscreen pass)
- `GL_EXT_shader_framebuffer_fetch` decodes the PQ framebuffer and blends in linear light
- Result: **same performance as SDR rendering** — no extra FBO bind or GPU sync

---

## Phase 1: SDR Color Intensity (Gamut Expansion) — ✅ COMPLETE

**Priority:** 🔴 CRITICAL  
**Impact:** Makes SDR content look vibrant on HDR displays (KWin's most praised HDR feature)  
**Status:** ✅ Implemented and tested

### What Was Done

1. **Config support** — `sdr_color_intensity: Option<f64>` in `HdrOutputConfig` (niri-ipc)
   - Parsed from KDL config: `hdr enabled=true sdr-color-intensity=1.2`
   - IPC support: `niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.5`
   - Range: 0.0–2.0, default 1.0

2. **HDR shader** (`src/render_helpers/shaders/hdr_output.frag`)
   - Per-element shader override via `override_default_tex_program()`
   - Pipeline: sRGB → linear → gamut expansion → scale to nits → BT.2020 → PQ
   - `GL_EXT_shader_framebuffer_fetch` for correct alpha blending in linear light
   - Fallback path (premultiplied PQ) when extension unavailable
   - Handles both premultiplied-alpha and NO_ALPHA texture variants
   - DEBUG_FLAGS support (tint)

3. **Rendering architecture** (`src/render_helpers/hdr_output.rs`, `src/backend/tty.rs`)
   - `HdrWrappedElement<'a>` wraps each `OutputRenderElements` with HDR shader override
   - Delegates all `Element` trait methods (id, geometry, damage, opaque_regions)
   - `RenderElement<TtyRenderer>` impl sets/clears shader override around inner draw
   - Eliminates offscreen texture — DRM compositor handles damage tracking natively

4. **Key bugs fixed**
    - sRGB→BT.2020 matrix was transposed (column-major vs row-major) → caused yellow color shift
    - Alpha blending in PQ space caused black screen with overlays → framebuffer fetch fix
    - Performance: eliminated two-pass offscreen architecture → single-pass per-element
    - IPC config changes didn't trigger redraw → added `resized_outputs.push()` + `reset_buffer_ages()` in config change path
    - **Massive memory leak (38GB OOM)** → `shaders::init()` recompiled GLSL shaders every frame. Fixed by making init idempotent (check if already compiled before compiling).

5. **Known issues**
    - [ ] **Cursor plane artifact** — small transparent square around mouse pointer persists even with `ALLOW_CURSOR_PLANE_SCANOUT` removed. Cursor elements are rendered as `Kind::Cursor` but still bypass the HDR shader via some other path (likely direct scanout or separate composition). Needs investigation.

---

## Phase 2: Per-Surface Color Awareness — 🚧 IN PROGRESS

**Priority:** 🟡 HIGH  
**Impact:** Avoids unnecessary HDR conversion for native HDR windows  
**Status:** ⚠️ Implemented but **NOT YET TESTED** on real HDR display. Color management protocol (2.1-2.2) pending.

### Testing TODO

- [ ] **Basic passthrough activation** — Launch mpv with HDR video, verify `HDR passthrough: activated` appears in log
- [ ] **Per-element treatment** — Open SDR window (e.g. terminal with opacity) over HDR video, verify SDR window colors are correct (not washed out or double-converted)
- [ ] **SDR-only mode** — Close HDR app, verify all elements switch back to `Convert` treatment
- [ ] **Popup handling** — Open mpv context menu or OSD overlay, verify popups render correctly
- [ ] **Multiple passthrough apps** — Configure `passthrough-apps="mpv,kodi"`, verify both apps trigger passthrough
- [ ] **IPC runtime change** — Run `niri msg output HDMI-A-1 hdr true --passthrough-apps mpv`, verify passthrough activates without restart
- [ ] **Performance** — Verify no FPS drop with per-element surface ID matching vs output-wide approach
- [ ] **Cursor artifact** — Verify cursor still renders correctly over passthrough content (known issue from Phase 1/4)

### What Was Done

1. **HDR passthrough shader** (`src/render_helpers/shaders/hdr_passthrough.frag`)
   - Simple pass-through for content already in PQ/BT.2020
   - Handles both premultiplied-alpha and NO_ALPHA variants
   - Used when native HDR content should not be double-converted

2. **Config support** — `passthrough_app: Vec<String>` in `HdrOutput`
   - Parsed from KDL config: `hdr enabled=true passthrough-app "mpv" passthrough-app "kodi"`
   - IPC support: `niri msg output HDMI-A-1 hdr true --passthrough-apps mpv,kodi`

3. **Rendering architecture** (`src/render_helpers/hdr_output.rs`)
   - `HdrWrappedElement` now supports `HdrTreatment::Convert` and `HdrTreatment::Passthrough`
   - `conversion_program` and `passthrough_program` both stored
   - Draw selects program based on treatment

### TODO

- [x] Implement app-id matching in `render_hdr_frame` to determine per-element treatment
- [ ] Wire up `ColorManagementState` surface tracking when color-management protocol is implemented
- [x] Handle mixed HDR/SDR content correctly (some elements convert, some passthrough)
- [x] Set output image description when HDR is enabled/disabled (2.4) ⚠️ **NOT TESTED**

### 2.1 Complete Color Management Protocol

**File:** `src/protocols/color_management.rs`

- Implement `wp_color_management_v1` protocol delegation
- Handle `wp_image_description_source_v1` for per-surface color info
- Track which surfaces declare HDR vs SDR content
- Store surface image descriptions in `ColorManagementState`

### 2.2 Protocol Handler Integration

**File:** `src/handlers/mod.rs`

- Implement `ColorManagementHandler` trait fully
- Delegate protocol dispatch to `ColorManagementState`
- Handle surface lifecycle (commit, destroy)

### 2.3 Rendering Path Optimization

**File:** `src/backend/tty.rs`

- Check surface image description before applying HDR conversion
- For HDR surfaces: pass through without conversion
- For SDR surfaces: apply SDR→HDR conversion with gamut expansion
- Handle mixed HDR/SDR content correctly

### 2.4 Output Image Description

**File:** `src/backend/tty.rs`, `src/niri.rs`

- ✅ Set output image description based on HDR config when HDR is enabled
- ✅ Remove output image description when HDR is disabled
- ⚠️ TODO: Wire up `transfer_function` and `colorspace` from config once HLG config fields are added to `niri_config::HdrOutput`
- ⚠️ TODO: Include mastering display info from EDID when available

**Status:** ⚠️ Implemented but **NOT YET TESTED** on real HDR display.

**How to test:**
1. Build and install the updated binary
2. Enable HDR on your display: `niri msg output HDMI-A-1 hdr true`
3. Verify no crashes or errors in the log
4. Disable HDR: `niri msg output HDMI-A-1 hdr false`
5. Verify no crashes or errors
6. **Code-level verification:** Add a temporary `warn!` log in `reload_output_config` to print the `ImageDescription` values when HDR is toggled, confirm they match expected defaults (PQ, BT.2020, 1000 nits max, 0.005 nits min)

---

## Phase 3: ICC Profile Support — ✅ COMPLETE

**Priority:** 🟡 MEDIUM  
**Impact:** Color-accurate SDR rendering on wide-gamut displays  
**Effort:** ~4-5 days  
**Status:** ✅ Implemented, tested, and merged into `feature/hdr-support`

### What Was Done

1. **ICC Profile Parser** (`src/color/icc.rs`)
   - Parse ICC v2 and v4 profiles (header, tag table, rXYZ/gXYZ/bXYZ/wtpt)
   - Extract color primaries and white point from profile
   - Compute sRGB→Display color correction matrix using linear algebra
   - Standard sRGB→XYZ(D65) + Bradford D65→D50 adaptation
   - Supports path expansion (`~` → `$HOME`)

2. **Config Support** (`niri-config/src/output.rs`)
   ```kdl
   output "HDMI-A-1" {
       icc-profile "/usr/share/color/icc/colord/sRGB.icc"
   }
   ```
   - `icc_profile: Option<String>` on output config
   - Loaded when output connects in `add_output()`
   - Stored parsed profile in `OutputState`

3. **ICC Profile Shader Integration** (`src/render_helpers/shaders/hdr_output.frag`)
   - `u_icc_enabled` (int) and `u_icc_matrix` (mat3) uniforms
   - Replaces BT.2020 matrix with ICC-derived matrix when enabled
   - Matrix passed as column-major 3×3 GLSL uniform
   - Applied in per-element render path

4. **IPC Toggle** (`niri-ipc/src/lib.rs`, `src/niri.rs`)
   - `niri msg output HDMI-A-1 icc true` — enable profile at runtime
   - `niri msg output HDMI-A-1 icc false` — disable profile at runtime
   - Only works when profile is configured in config; unspecified fields retain previous values

5. **Critical Bug Fix — Clipped Windows** (`src/render_helpers/clipped_surface.rs`)
   - **Problem:** `ClippedSurfaceRenderElement` (rounded corners) overwrote HDR shader override with its own corner-clipping shader, causing ICC/hdr to bypass on browser/terminal windows
   - **Fix:** Smithay patch to make `tex_program_override` stackable (`Vec` push/pop). `ClippedSurfaceRenderElement` now appends corner uniforms to existing HDR override when active.
   - `hdr_output.frag` merged with `rounding_alpha.frag` for conditional corner clipping
   - `HdrWrappedElement` passes default no-op corner uniforms for non-clipped surfaces

6. **Shader Memory Leak Fix**
   - `shaders::init()` was recompiling GLSL every frame → 38GB OOM crash
   - Made idempotent by checking if shaders already exist before compiling

### Testing Results

- ✅ Build: `cargo build --release` successful
- ✅ Test on real HDR display: ICC produces visible color difference (pure red shifts measurably)
- ✅ Semi-transparent windows: No regression
- ✅ Clipped windows (browser, terminal): ICC now applies correctly after stackable override fix
- ✅ IPC partial updates: `niri msg output HDMI-A-1 icc true` works, queues redraw automatically
- ✅ Gamma control disabled when HDR active

### Files Added/Modified

- **New:** `src/color/icc.rs` — ICC parser and matrix computation
- **New:** `src/color/mod.rs` — Color module root
- **New:** `patches/smithay-tex-program-override-stack.patch` — Smithay patch doc
- **Modified:** `src/render_helpers/shaders/hdr_output.frag` — Added ICC uniforms + corner clipping
- **Modified:** `src/render_helpers/shaders/mod.rs` — Compile `rounding_alpha.frag` into `hdr_output`
- **Modified:** `src/render_helpers/hdr_output.rs` — Pass default no-op corner uniforms
- **Modified:** `src/render_helpers/clipped_surface.rs` — Append uniforms instead of replacing program
- **Modified:** `src/backend/tty.rs` — ICC matrix retrieval, dynamic metadata
- **Modified:** `src/niri.rs` — `add_output()` ICC loading, `apply_transient_output_config()` ICC toggle, `OutputState`
- **Modified:** `niri-config/src/output.rs` — `icc-profile` child node
- **Modified:** `niri-ipc/src/lib.rs` — `OutputAction::Icc` enum

- ✅ Add `icc_profile: Option<String>` to output config
- ✅ Load profile when output connects
- ✅ Store parsed profile in `OutputState`

### 3.3 ICC Profile Shader Integration

**Files:** `src/render_helpers/shaders/hdr_output.frag`, `src/render_helpers/hdr_output.rs`

- ✅ Replace BT.2020 matrix with ICC-derived matrix when profile loaded
- ✅ Uniform `u_icc_enabled` and `u_icc_matrix` added to HDR shader
- ✅ Matrix passed as column-major 3×3 GLSL uniform

### 3.4 Integration

**Files:** `src/backend/tty.rs`, `src/niri.rs`

- ✅ Load ICC profile in `add_output()` when connector connects
- ✅ Pass ICC matrix to `HdrWrappedElement` during rendering
- ✅ Apply matrix when `icc_profile` is configured in output config

### Usage

```bash
# 1. Place ICC profile somewhere accessible
# 2. Add to config.kdl:
# output "DP-3" {
#     icc-profile "/home/user/.local/share/icc/my-display.icc"
# }
# 3. Reload config: niri msg action reload-config
```

---

## Phase 4: Gamut Mapping

**Priority:** 🟡 MEDIUM  
**Impact:** Prevents oversaturation of wide-gamut SDR content  
**Effort:** ~2-3 days  
**Status:** ✅ Merged into `feature/hdr-support` (`ceffb117`)

### 4.1 Gamut Mapping Modes

**File:** `niri-config/src/output.rs`

```kdl
output "HDMI-A-1" {
    hdr {
        gamut_mapping "desaturate"  // "clip", "desaturate", "relative"
    }
}
```

- ✅ Implement desaturate mode (KWin default)
- ✅ Implement clip mode (simple clamping)
- ✅ Implement relative mode (preserve relationships)
- ✅ IPC support: `niri msg output HDMI-A-1 hdr true --gamut-mapping desaturate`

### 4.2 Shader Implementation

**File:** `src/render_helpers/shaders/hdr_output.frag`

- ✅ `gamut_map()` function with 3 modes
- ✅ Uniform `u_gamut_mapping_mode` registered in `shaders/mod.rs`
- ✅ Pipeline order: `expand_gamut` → `srgb_to_bt2020` → `gamut_map` → scale to nits
- ✅ Framebuffer fetch for correct alpha blending in linear light

### Known Issues

- [ ] **Cursor plane artifact** — small transparent square around mouse pointer persists even with `ALLOW_CURSOR_PLANE_SCANOUT` removed. Cursor elements are rendered as `Kind::Cursor` but still bypass the HDR shader via some other path (likely direct scanout or separate composition). Needs investigation.

---

## Phase 5: Dynamic Metadata

**Priority:** 🟢 LOW  
**Impact:** Display can optimize brightness per-frame  
**Effort:** ~2 days  
**Status:** ✅ Implemented (Config-driven dynamic updates)

### 5.1 Dynamic Metadata Updates

**File:** `src/backend/tty.rs`

- ✅ Re-applies HDR metadata blob when config changes via IPC.
- ✅ Allows runtime updates of `max_cll`, `max_fall`, `max_luminance`.
- ⚠️ **Note:** Automatic per-frame pixel sampling for `max_cll` is not implemented.
  - The current GLES per-element architecture makes framebuffer readbacks expensive.
  - Dynamic updates are driven by config changes (e.g., external scripts or user commands).
  - Future work: Implement efficient metadata calculation via Vulkan compute or DMA-BUF analysis.

### Usage

```bash
# Update max_cll dynamically
niri msg output HDMI-A-1 hdr true --max-cll 4000
```

---

## Phase 6: HLG Support

**Priority:** 🟢 LOW  
**Impact:** Support broadcast HDR content  
**Effort:** ~1 day  
**Status:** ✅ Merged into `feature/hdr-support`

### 6.1 Config Support

```kdl
output "HDMI-A-1" {
    hdr {
        enabled true
        transfer_function "hlg"  // "pq" (default) or "hlg"
    }
}
```

### 6.2 Shader Implementation

**File:** `src/render_helpers/shaders/hdr_output.frag`

- HLG OETF/EOTF (ARIB STD-B67 / ITU-R BT.2100) added alongside PQ
- `u_transfer_function` uniform (0=PQ, 1=HLG) switches encoding/decoding
- Framebuffer fetch decodes using current transfer function
- Single shader handles both modes (no separate shader files needed)

### 6.3 DRM Metadata

**File:** `src/backend/tty.rs`

- `HdrOutputMetadata::new` accepts `transfer_function` parameter
- EOTF field set to 1 for HLG, 2 for PQ (per HDMI 2.1 / CTA-861-H)
- IPC support: `niri msg output HDMI-A-1 hdr true --transfer-function hlg`

---

## Phase 7: HDR Content Detection

**Priority:** 🟢 LOW  
**Impact:** Automatic HDR mode switching  
**Effort:** ~2-3 days

### 7.1 Content Analysis

**File:** `src/backend/tty.rs`

- Analyze rendered frame for HDR content indicators
- Detect PQ/HLG transfer function in content
- Track percentage of HDR vs SDR pixels

### 7.2 Auto HDR Mode

- Auto-enable HDR when HDR content detected
- Auto-disable HDR when only SDR content present
- Configurable thresholds and delays

---

## Phase 8: Vulkan HDR Layer

**Priority:** 🔵 FUTURE  
**Impact:** Games can output native HDR  
**Effort:** ~1-2 weeks

### 8.1 Vulkan Layer Implementation

- Implement `VK_EXT_swapchain_colorspace` support
- Create Vulkan layer for HDR passthrough
- Handle `VK_EXT_hdr_metadata` extension

### 8.2 Integration

- Detect Vulkan HDR surfaces
- Bypass compositor conversion for native HDR
- Handle fallback for non-HDR Vulkan apps

---

## Phase 9: Xwayland HDR

**Priority:** 🔵 FUTURE  
**Impact:** X11 apps can use HDR  
**Effort:** ~1 week

### 9.1 X11 Atom API

- Define X11 atoms for HDR signaling
- Handle X11 window HDR requests
- Convert X11 HDR content appropriately

### 9.2 Integration

- Detect Xwayland HDR windows
- Apply appropriate color transformation
- Handle mixed X11/Wayland HDR content

---

## Phase 10: HDR Calibration Wizard — 🚧 IN PROGRESS

**Priority:** 🔵 FUTURE  
**Impact:** User-friendly HDR setup  
**Effort:** ~3-4 days

### 10.1 Calibration UI — ⏳ NOT STARTED

- Visual test patterns for HDR calibration (black level, peak white, gray ramp, primaries)
- Step-through wizard via IPC or keybinding
- Fine-tuned recommendations beyond EDID defaults
- Save calibration results to config

### 10.2 EDID Parsing — ✅ COMPLETE (Phase 10A)

**Status:** ✅ Implemented, tested on real hardware

**Files:** `src/calibration/edid.rs`, `src/backend/tty.rs`

- Parse display EDID for HDR capabilities via `libdisplay_info::Info::hdr_static_metadata()`
- Extract: `desired_content_max_luminance`, `desired_content_min_luminance`, `desired_content_max_frame_avg_luminance`
- Detect supported EOTFs: PQ (traditional_hdr), HLG (hlg)
- Auto-configure optimal settings when config omits `max-luminance`, `min-luminance`, `max-cll`, `max-fall`
- Display capabilities logged on output connect with suggested KDL config block
- Config values always take priority over EDID (allows user override)

**Testing Results:**
- ✅ DP-3 (ultrawide): EDID correctly detected `max_lum=1015.2, min_lum=0.051, max_fall=603.7`
- ✅ HDMI-A-1 (LG OLED 42"): EDID reports no HDR Static Metadata block — manual config required
- ✅ IPC changes correctly isolated per-output
- ✅ Config reload path verified working

**Config Syntax Change:**
HDR properties are now child nodes (not inline properties):
```kdl
output "HDMI-A-1" {
    hdr {
        enabled
        max-luminance 730.0
        min-luminance 0.0005
        max-cll 730.0
        max-fall 350.0
        sdr-brightness 300.0
        sdr-color-intensity 1.2
        passthrough-apps "mpv,kodi,firefox"
        gamut-mapping "desaturate"
    }
}
```

**Known Issues:**
- Duplicate `output` blocks in config (e.g., main file + include) cause last-one-wins behavior
- Some OLED TVs over HDMI omit HDR metadata in EDID despite supporting HDR

### Usage

```bash
# Check EDID-detected capabilities in logs:
journalctl --user -f -u niri | grep "EDID HDR"

# Minimal config (uses EDID defaults):
output "DP-3" {
    hdr { enabled }
}

# Manual config for displays with incomplete EDID:
output "HDMI-A-1" {
    hdr {
        enabled
        max-luminance 730.0
        min-luminance 0.0005
        sdr-brightness 300.0
    }
}
```

---

## Implementation Order Recommendation

1. **Phase 1** - SDR Color Intensity (immediate usability improvement)
2. **Phase 2** - Per-Surface Color Awareness (foundation for advanced features)
3. **Phase 4** - Gamut Mapping (complements Phase 1)
4. **Phase 3** - ICC Profile Support (color accuracy)
5. **Phase 6** - HLG Support (completes HDR format support)
6. **Phase 5** - Dynamic Metadata (optimization)
7. **Phase 7** - HDR Content Detection (automation)
8. **Phase 8-10** - Advanced features (Vulkan, Xwayland, Calibration)

---

## Files Modified Summary

| Phase | New Files | Modified Files |
|-------|-----------|----------------|
| 1 | - | `hdr_output.frag`, `hdr_output.rs`, `tty.rs`, `output.rs`, `lib.rs` |
| 2 | - | `color_management.rs`, `mod.rs` (handlers), `tty.rs` |
| 3 | `icc.rs`, `color/mod.rs` | `output.rs`, `lib.rs`, `tty.rs`, `hdr_output.frag`, `hdr_output.rs`, `clipped_surface.rs`, `shaders/mod.rs`, `niri.rs` |
| 4 | - | `hdr_output.frag`, `hdr_output.rs`, `tty.rs`, `niri.rs`, `output.rs`, `lib.rs`, `shaders/mod.rs` |
| 5 | - | `tty.rs` |
| 6 | - | `hdr_output.frag`, `hdr_output.rs`, `tty.rs`, `niri.rs`, `output.rs`, `lib.rs`, `shaders/mod.rs` |
| 10 | `calibration/edid.rs`, `calibration/mod.rs` | `tty.rs`, `niri.rs`, `Cargo.toml` |

### External Patches

| Patch | Location | Purpose |
|-------|----------|---------|
| `smithay-tex-program-override-stack.patch` | `.cargo/git/checkouts/smithay-*/src/backend/renderer/gles/mod.rs` | Makes `tex_program_override` stackable for clipped window + HDR coexistence |

---

## Testing Strategy

### Automated Tests
- Unit tests for color conversion functions
- Integration tests for config parsing
- Protocol compliance tests

### Manual Tests
- Visual comparison with KWin HDR output
- Test patterns for color accuracy
- Performance benchmarks (FPS impact)
- Multi-monitor HDR/SDR mixing

### Test Content
- SDR desktop content (web browser, terminal)
- HDR video content (YouTube HDR, local files)
- HDR games (via Vulkan layer when available)
- Wide-gamut images (P3, Adobe RGB)

---

## Performance Considerations

The per-element shader override architecture provides optimal performance:

- **No offscreen texture** — the DRM compositor renders directly to the swapchain
- **Native damage tracking** — only changed elements are redrawn (DRM compositor handles this)
- **Single render pass** — same as SDR rendering, no FBO bind/unbind overhead
- **Framebuffer fetch** — `GL_EXT_shader_framebuffer_fetch` reads the framebuffer for
  correct alpha blending, which is essentially free (L1 cache hit on modern GPUs)
- **No GPU sync** — no implicit sync between render passes

Previous architectures tried and rejected:
- **Offscreen texture + full-screen HDR quad**: Caused severe performance issues because
  any pixel change resulted in full-screen damage for the single HdrOutputRenderElement,
  defeating the DRM compositor's per-element damage optimization.
- **Offscreen with damage tracking**: Still slow due to FBO bind overhead and the
  fundamental mismatch between element-level damage tracking and single-element output.

---

## Branch Status

| Branch | Base / Head | Purpose |
|--------|-------------|---------|
| `feature/hdr-support` | `8a73d7fd` (squashed Phase 1 + 2 + 4 + 6) | Stable integration branch — always contains clean, logical milestones |
| `feature/hdr-sdr-intensity` | `6c2ba2cd` (original 9 commits) | Development branch — full trial-and-error history preserved |
| `feature/hdr-color-aware` | `cf45cbf3` | Phase 2: Per-surface color awareness (rebased) |
| `feature/hdr-gamut-mapping` | `fc3121d1` (Phase 4 complete) | Phase 4: Gamut mapping (✅ merged into hdr-support) |
| `feature/hdr-icc-profiles` | `cf45cbf3` | Phase 3: ICC profile support (rebased) |
| `feature/hdr-dynamic-meta` | `0cad31be` | Phase 5: Dynamic metadata (✅ implemented, ⚠️ untested) |
| `feature/hdr-hlg` | `80960ce0` | Phase 6: HLG support (✅ merged into hdr-support) |
| `feature/hdr-calibration-wizard` | current | Phase 10: Calibration wizard (10A EDID ✅ complete, 10.1 UI ⏳ pending) |

---

## Git Workflow for HDR Feature Branches

**Rule:** `feature/hdr-support` must never contain fixup/debug/refactor commits. It only receives clean, squashed milestones.

### Adding a new Phase

```bash
# Start from the stable integration branch
git checkout feature/hdr-support

# Create/work on the feature branch
git checkout -b feature/hdr-XYZ
# ... hack, commit fixups, debug, refactor ...

# When the phase is complete, squash-merge into hdc-support
git checkout feature/hdr-support
git merge --squash feature/hdr-XYZ
git commit -m "feat(HDR): Phase N — description"
git push --force-with-lease origin feature/hdr-support

# Then rebase all dependent feature branches
for b in feature/hdr-color-aware feature/hdr-gamut-mapping ...; do
  git checkout $b
  git rebase feature/hdr-support
  git push --force-with-lease origin $b
done
```

### Why this matters

- `feature/hdr-support` is a **clean milestone branch** — future readers see logical progress
- The individual feature branches preserve full development history (debug commits, fixups, failed approaches)
- If the original feature branch already pushed its full history (e.g. `feature/hdr-sdr-intensity`), it stays untouched — only `hdr-support` gets squashed

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Shader complexity | Performance drop | Keep shaders simple, profile regularly |
| Protocol incompatibility | Apps don't work | Test with major apps (Firefox, MPV, games) |
| Display quirks | Wrong colors | Add display-specific overrides |
| Driver bugs | Crashes/glitches | Graceful fallback to SDR |
| User confusion | Bad experience | Clear documentation, calibration wizard |
