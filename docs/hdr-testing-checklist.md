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

### Known Issues
- [ ] Cursor artifact: Present / Absent — Notes:
- [ ] Memory stable: Yes / No — Notes:
- [ ] Transparency correct: Yes / No — Notes:
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
