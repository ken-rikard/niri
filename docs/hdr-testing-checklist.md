# HDR Feature Testing Checklist

This document lists all HDR features that require testing on a real HDR display.

## Prerequisites

- HDR-capable display connected via HDMI 2.1 or DP 1.4+
- Display HDR mode enabled in display OSD
- niri built from `feature/hdr-dynamic-meta` branch
- `cargo build --release` completed successfully
- Launch niri from SDDM or tty

### How to Verify HDR is Active

```bash
# Check HDR status
niri msg output HDMI-A-1 hdr

# Enable HDR
niri msg output HDMI-A-1 hdr true

# Enable HDR with options
niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.2 --gamut-mapping desaturate
```

### How to Monitor Logs

```bash
# If launched from SDDM
journalctl --user -f -u niri

# Or check session log
tail -f ~/.local/share/sddm/wayland-session.log
```

---

## Phase 1: SDR Color Intensity — ✅ Previously Tested

**Branch:** `feature/hdr-support`

### Test 1.1: Basic SDR Color Intensity

1. Enable HDR: `niri msg output HDMI-A-1 hdr true`
2. Adjust intensity: `niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.5`
3. **Verify:** SDR content (desktop, browser, terminal) appears more vibrant
4. **Verify:** No color banding or artifacts
5. Test range: 0.5 (muted) → 1.0 (normal) → 2.0 (vivid)

### Test 1.2: SDR Color Intensity via Config

1. Add to `config.kdl`:
   ```kdl
   output "HDMI-A-1" {
       hdr {
           enabled true
           sdr-color-intensity 1.3
       }
   }
   ```
2. Reload config: `niri msg action reload-config`
3. **Verify:** Setting applied without visual glitch

---

## Phase 2: Per-Surface Color Awareness — ✅ TESTED

**Branch:** `feature/hdr-color-aware`

### Test 2.1: Per-Element Passthrough

**Purpose:** Native HDR content (video players) should bypass SDR→HDR conversion.

**Status:** ✅ Working

1. Enable HDR: `niri msg output HDMI-A-1 hdr true`
2. Open a native HDR video (e.g., HDR YouTube video in browser, or mpv with HDR content)
3. Open an SDR window (terminal, file manager) on top of the HDR video
4. **Verify:** HDR video retains its native HDR appearance
5. **Verify:** SDR window is properly converted to HDR (not washed out or oversaturated)
6. **Verify:** Transparency/opacity in SDR windows blends correctly over HDR content

### Test 2.2: Passthrough App Configuration

1. Add to `config.kdl`:
   ```kdl
   hdr {
       passthrough-apps ["mpv" "firefox"]
   }
   ```
2. Launch mpv with HDR video
3. **Verify:** mpv window uses passthrough shader (native HDR)
4. Launch a non-passthrough app (e.g., alacritty)
5. **Verify:** alacritty uses SDR→HDR conversion shader

### Test 2.3: Mixed Content Layering

1. Enable HDR with passthrough for a video player
2. Stack multiple windows: HDR video → SDR browser → semi-transparent terminal
3. **Verify:** Each layer renders correctly with appropriate treatment
4. **Verify:** No flickering or corruption when moving windows

---

## Phase 3: ICC Profile Support — ✅ TESTED

**Branch:** `feature/hdr-icc-profiles`

**Note:** ICC correction is applied in the HDR shader path. When HDR is disabled, the profile currently has no effect (SDR pipeline uses direct rendering without color management).

### Test 3.1: ICC Profile Loading

**Status:** ✅ Working

1. Obtain or generate an ICC profile for your display:
   - sRGB profile: `/usr/share/color/icc/colord/sRGB.icc`
   - Display P3 profile (for wide-gamut displays)
   - Calibrated profile from colorimeter
2. Add to `config.kdl`:
   ```kdl
   output "DP-3" {
       icc-profile "/path/to/profile.icc"
       hdr enabled=true
   }
   ```
3. Reload config: `niri msg action reload-config`
4. Check logs for: `Loaded ICC profile for DP-3: /path/to/profile.icc (Description)`
5. **Verify:** No errors during profile loading

### Test 3.2: Color Accuracy

**Status:** ✅ Visible difference confirmed

1. With ICC profile loaded and HDR enabled, display a known sRGB test pattern (solid pure red `#FF0000`)
2. **Verify:** Colors shift measurably compared to without ICC
3. For Display P3 monitors: verify sRGB content is NOT oversaturated
4. Compare with and without ICC profile by commenting out the config line

### Test 3.3: IPC Toggle

**Status:** ✅ Working

1. With ICC configured and HDR enabled:
   ```bash
   niri msg output HDMI-A-1 icc false
   niri msg output HDMI-A-1 icc true
   ```
2. **Verify:** Toggle takes effect immediately (redraw queued automatically)
3. **Verify:** No crashes or visual glitches

### Test 3.4: Clipped Windows (Browser, Terminal)

**Status:** ✅ Fixed and verified

1. Open browser or terminal window (they use `ClippedSurfaceRenderElement` for rounded corners)
2. Enable ICC: `niri msg output HDMI-A-1 icc true`
3. **Verify:** ICC effect is visible on clipped windows (not just on dock/bar)
4. **Regression check:** Disable ICC, verify colors return to normal

**Note:** This test was failing before the Smithay stackable-override patch. ClippedSurfaceRenderElement was replacing the HDR shader with its corner-clipping shader, bypassing ICC entirely.

---

## Phase 4: Gamut Mapping — ✅ Previously Tested

**Branch:** `feature/hdr-support`

### Test 4.1: Gamut Mapping Modes

1. Open a wide-gamut image (Display P3 or Adobe RGB)
2. Test each mode:
   ```bash
   niri msg output HDMI-A-1 hdr true --gamut-mapping clip
   niri msg output HDMI-A-1 hdr true --gamut-mapping desaturate
   niri msg output HDMI-A-1 hdr true --gamut-mapping relative
   ```
3. **Verify:** `clip` — oversaturated colors are clamped, may lose detail
4. **Verify:** `desaturate` — colors are desaturated to fit BT.2020, preserves detail
5. **Verify:** `relative` — color relationships preserved, overall saturation reduced

### Test 4.2: Gamut Mapping with SDR Color Intensity

1. Combine both features:
   ```bash
   niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.5 --gamut-mapping desaturate
   ```
2. **Verify:** Gamut expansion and mapping work together without artifacts

---

## Phase 5: Dynamic Metadata — ✅ TESTED

**Branch:** `feature/hdr-dynamic-meta`

### Test 5.1: Dynamic max_cll Update

**Status:** ✅ Working

1. Enable HDR: `niri msg output HDMI-A-1 hdr true`
2. Check current metadata: `niri msg output HDMI-A-1 hdr`
3. Update max_cll: `niri msg output HDMI-A-1 hdr true --max-cll 4000`
4. **Verify:** Logs show new HDR blob created with different metadata values
5. **Verify:** No visual flicker or glitch during update

**Note:** Most consumer displays ignore dynamic metadata and use static EDID values. No visual difference is expected.

### Test 5.2: Dynamic max_fall Update

1. Update max_fall: `niri msg output HDMI-A-1 hdr true --max-fall 1600`
2. **Verify:** No visual artifacts

### Test 5.3: Combined Metadata Update

1. Update multiple fields at once:
   ```bash
   niri msg output HDMI-A-1 hdr true --max-cll 1000 --max-fall 400 --sdr-color-intensity 1.2
   ```
2. **Verify:** All fields update correctly
3. **Verify:** Unspecified fields retain their previous values (partial update)

### Test 5.4: Config-Driven Metadata Update

1. Add to `config.kdl`:
   ```kdl
   output "HDMI-A-1" {
       hdr {
           enabled true
           max-cll 2000
           max-fall 800
       }
   }
   ```
2. Reload config: `niri msg action reload-config`
3. **Verify:** Metadata updated to match config values

---

## Phase 6: HLG Support — 🟡 DISPLAY-DEPENDENT

**Branch:** `feature/hdr-support`

**Status:** ✅ Code implemented and functional — Display did NOT support HLG

**Note:** HLG requires a display that supports the HLG transfer function. Most consumer displays only support HDR10 (PQ). When HLG mode is selected on a non-HLG display, the display typically disables HDR or falls back to SDR.

### Test 6.1: HLG Transfer Function

1. Enable HLG mode:
   ```bash
   niri msg output HDMI-A-1 hdr true --transfer-function hlg
   ```
2. **Verify:** Logs show `transfer_function=1, eotf=1` (HLG metadata)
3. If display supports HLG: SDR content renders with HLG encoding
4. If display does NOT support HLG: Screen gets darker or HDR disables

### Test 6.2: PQ vs HLG Switching

1. Start with PQ: `niri msg output HDMI-A-1 hdr true --transfer-function pq`
2. Switch to HLG: `niri msg output HDMI-A-1 hdr true --transfer-function hlg`
3. Switch back to PQ: `niri msg output HDMI-A-1 hdr true --transfer-function pq`
4. **Verify:** No visual glitches during switching
5. **Verify:** Each mode renders correctly after switch (if display supports both)

### Test 6.3: HLG with Gamut Mapping

1. Enable HLG with gamut mapping:
   ```bash
   niri msg output HDMI-A-1 hdr true --transfer-function hlg --gamut-mapping desaturate
   ```
2. **Verify:** Gamut mapping works correctly with HLG transfer function

---

## Known Issues to Verify

### Cursor Plane Artifact

1. Enable HDR on any mode (PQ or HLG)
2. Move mouse cursor around the screen
3. **Check:** Is there a small transparent square around the cursor?
4. **Check:** Does it appear on all content types (SDR windows, HDR video, desktop)?
5. **Note:** If present, document GPU/driver combination for investigation

### Memory Stability

1. Enable HDR and use normally for 30+ minutes
2. Monitor memory usage: `watch -n 5 'ps aux | grep niri'`
3. **Verify:** Memory usage remains stable (no continuous growth)
4. **Verify:** No OOM crashes

### Semi-Transparent Windows

1. Open a terminal with opacity set (e.g., 80% transparent)
2. Move it over various content (desktop, browser, video)
3. **Verify:** Transparency blends correctly, no color shifts or artifacts
4. **Verify:** Text remains readable

---

## Phase 10: HDR Calibration Wizard — EDID Auto-Config ✅ TESTED

**Branch:** `feature/hdr-calibration-wizard`

### Test 10.1: EDID HDR Detection

**Status:** ✅ Working

1. Connect an HDR-capable display
2. Check niri logs for:
   ```
   EDID HDR capabilities for DP-3: max_lum=1015.2, min_lum=0.051, max_fall=603.7, pq=true, hlg=false, hdr10=true
   ```
3. **Verify:** Values match display's advertised capabilities
4. **Verify:** Suggested config block is logged

### Test 10.2: Defaults from EDID

**Status:** ✅ Working

1. Use minimal HDR config (no luminance values):
   ```kdl
   output "DP-3" {
       hdr { enabled }
   }
   ```
2. Enable HDR: `niri msg output DP-3 hdr true`
3. Check logs for actual values used in DRM metadata commit
4. **Verify:** `max_luminance`, `min_luminance`, `max_cll`, `max_fall` match EDID values
5. **Verify:** Transfer function defaults to PQ unless display only supports HLG

### Test 10.3: Config Override

**Status:** ✅ Working

1. Override EDID values in config:
   ```kdl
   output "DP-3" {
       hdr {
           enabled
           max-luminance 1200.0
           min-luminance 0.001
       }
   }
   ```
2. **Verify:** Config values take precedence over EDID defaults
3. **Verify:** Logs show overridden values in DRM metadata

### Test 10.4: Manual Override for Displays with Incomplete EDID

**Status:** ✅ Tested (LG OLED 42" over HDMI)

Some HDR-capable displays (notably OLED TVs over HDMI) do **not** expose the CTA-861 HDR Static Metadata block in their EDID. When this happens, niri logs:

```
EDID parsed for HDMI-A-1 but no HDR Static Metadata block found (or max_luminance == 0)
```

In this case, you **must** specify HDR values manually in `config.kdl`.

**Example: LG OLED 42" (measured with DisplayCAL)**

| Parameter | Value | Source |
|-----------|-------|--------|
| `max-luminance` | 730 | Spec sheet peak brightness (cd/m²) |
| `min-luminance` | 0.0005 | OLED near-perfect black |
| `max-cll` | 730 | Same as max-luminance |
| `max-fall` | 350 | ~95% of sustained brightness (360 cd/m²) |
| `sdr-brightness` | 300 | Brighter SDR reference white for OLEDs |
| `sdr-color-intensity` | 1.2 | Moderate gamut expansion (panel is 130% sRGB) |
| `gamut-mapping` | desaturate | Prevents oversaturation |

```kdl
output "HDMI-A-1" {
    mode "3840x2160@120"
    scale 1.0
    variable-refresh-rate

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

**Verify:**
1. Display's HDR info overlay shows active HDR mode
2. Black levels are OLED-perfect (no raised blacks)
3. SDR content (browser, terminal) is readable
4. HDR video (mpv `--vo=gpu-next`) shows correct peak brightness
5. No EDID-related warnings when config values are present

**Important:** Avoid duplicate `output` blocks across config files. Use a single definition per output.

---

## Test Results Template

Copy this section and fill in results:

```markdown
## Test Results

**Date:** YYYY-MM-DD
**Display:** [Model, connection type]
**GPU/Driver:** [GPU model, driver version]
**Branch tested:** [branch name, commit hash]

### Phase 1: SDR Color Intensity
- [ ] Test 1.1: PASS / FAIL — Notes:
- [ ] Test 1.2: PASS / FAIL — Notes:

### Phase 2: Per-Surface Color Awareness
- [ ] Test 2.1: PASS / FAIL — Notes:
- [ ] Test 2.2: PASS / FAIL — Notes:
- [ ] Test 2.3: PASS / FAIL — Notes:

### Phase 3: ICC Profile Support
- [ ] Test 3.1: PASS / FAIL — Notes:
- [ ] Test 3.2: PASS / FAIL — Notes:

### Phase 4: Gamut Mapping
- [ ] Test 4.1: PASS / FAIL — Notes:
- [ ] Test 4.2: PASS / FAIL — Notes:

### Phase 5: Dynamic Metadata
- [ ] Test 5.1: PASS / FAIL — Notes:
- [ ] Test 5.2: PASS / FAIL — Notes:
- [ ] Test 5.3: PASS / FAIL — Notes:
- [ ] Test 5.4: PASS / FAIL — Notes:

### Phase 6: HLG Support
- [ ] Test 6.1: PASS / FAIL — Notes:
- [ ] Test 6.2: PASS / FAIL — Notes:
- [ ] Test 6.3: PASS / FAIL — Notes:

### Phase 10: EDID Auto-Config
- [ ] Test 10.1: PASS / FAIL — Display advertises HDR capabilities in EDID log
- [ ] Test 10.2: PASS / FAIL — Default values match EDID when config omits luminance fields
- [ ] Test 10.3: PASS / FAIL — Config values override EDID correctly
- [ ] Test 10.4: PASS / FAIL — Manual config works for displays with incomplete EDID

### Per-Display Notes
**HDMI-A-1 (LG OLED 42"):**
- EDID HDR block: Present / Absent
- Recommended max-luminance: 
- Recommended sdr-brightness: 
- Tone mapping: Acceptable / Too dim / Too bright

**DP-3 (Ultrawide):**
- EDID HDR block: Present / Absent
- EDID max-luminance: 
- Working config:

### Known Issues
- [ ] Cursor artifact: Present / Absent — Notes:
- [ ] Memory stable: Yes / No — Notes:
- [ ] Transparency correct: Yes / No — Notes:
- [ ] Clipped windows (ICC): PASS / FAIL — Notes:
- [ ] Multi-output config isolation: PASS / FAIL — Notes:
```

---

## Quick Smoke Test (5 minutes)

If you only have a few minutes, run these essential checks:

```bash
# 1. Enable HDR
niri msg output HDMI-A-1 hdr true

# 2. Adjust SDR intensity
niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.5

# 3. Test gamut mapping
niri msg output HDMI-A-1 hdr true --gamut-mapping desaturate

# 4. Test dynamic metadata
niri msg output HDMI-A-1 hdr true --max-cll 4000

# 5. Test HLG
niri msg output HDMI-A-1 hdr true --transfer-function hlg

# 6. Switch back to PQ
niri msg output HDMI-A-1 hdr true --transfer-function pq

# 7. Disable HDR
niri msg output HDMI-A-1 hdr false
```

**Verify after each command:**
- Display recognizes the change (check HDR info overlay)
- No visual glitches or flickering
- niri remains responsive
