# Niri HDR Development — Project Context

This is the fork `ken-rikard/niri` implementing HDR support for the niri Wayland compositor.

## Repository

- **Origin:** `https://github.com/ken-rikard/niri.git`
- **Upstream:** `https://github.com/YaLTeR/niri.git`

## HDR Feature Branch Structure

| Branch | Role |
|--------|------|
| `main` | **Fork default branch.** Contains all HDR features + full dev history + documentation. This is the fork's showcase branch. |
| `feature/hdr-support` | **PR-ready branch** for upstream submission. Based on latest `upstream/main`. Clean single-merge commit with no internal dev files or debug history. |
| `feature/hdr-calibration-wizard` | Current development branch. Based on `feature/hdr-support`. 🚧 **IN PROGRESS** (Phase 10: HDR calibration wizard). |
| `feature/hdr-sdr-intensity` | Historical Phase 1 dev branch. Content already merged into `main`. |
| `feature/hdr-color-aware` | Historical Phase 2 dev branch. Content already merged into `main`. |
| `feature/hdr-gamut-mapping` | Historical Phase 4 dev branch. Content already merged into `main`. |
| `feature/hdr-dynamic-meta` | Historical Phase 5 dev branch. Content already merged into `main`. |
| `feature/hdr-hlg` | Historical Phase 6 dev branch. Content already merged into `main`. |
| `feature/hdr-icc-profiles` | Historical Phase 3 dev branch. Content already merged into `main`. |

## Git Workflow

### To prepare an upstream PR from `feature/hdr-support`:

```bash
# Rebase on latest upstream/main
git checkout feature/hdr-support
git pull --rebase upstream main

# Force-push to update the PR
git push --force-with-lease origin feature/hdr-support
```

### To develop a new feature on `feature/hdr-calibration-wizard`:

```bash
# Create dev branch from hdr-support
git checkout -b feature/my-new-feature feature/hdr-support

# When ready, merge back into calibration-wizard
git checkout feature/hdr-calibration-wizard
git merge --no-ff feature/my-new-feature
```

### To sync `main` with upstream:

```bash
git fetch upstream
git checkout main
git rebase upstream/main
# Fix conflicts, then:
git push --force-with-lease origin main
```

## Key Files

- `docs/hdr-implementation-plan.md` — Master plan with phase descriptions, branch status, and this workflow
- `docs/hdr-testing-checklist.md` — Testing procedures and results
- `src/render_helpers/shaders/hdr_output.frag` — Core HDR shader (PQ EOTF, BT.2020, gamut expansion, ICC matrix)
- `src/render_helpers/hdr_output.rs` — Per-element HDR shader override (`HdrWrappedElement`)
- `src/backend/tty.rs` — HDR output pipeline, atomic commit, DRM metadata, EDID parsing
- `niri-config/src/output.rs` — HDR config parsing (KDL)
- `niri-ipc/src/lib.rs` — IPC types for runtime HDR control
- `src/color/icc.rs` — ICC profile parser and color correction matrix computation
- `src/calibration/edid.rs` — EDID HDR auto-configuration (Phase 10A)

## Architecture

- **Single-pass per-element rendering** — No offscreen texture. Each element is wrapped with `HdrWrappedElement` which calls `override_default_tex_program()` before drawing.
- **Framebuffer fetch for alpha blending** — `GL_EXT_shader_framebuffer_fetch` decodes PQ framebuffer and blends in linear light. Fallback to premultiplied-PQ path on older GPUs.
- **Stackable shader overrides** — `ClippedSurfaceRenderElement` (rounded corners) now appends uniforms to an existing HDR override instead of replacing it. Requires Smithay patch (`patches/smithay-tex-program-override-stack.patch`).
- **ICC profile color correction** — Replaces BT.2020 matrix with ICC-derived matrix when profile is loaded. Supports v2/v4 ICC profiles via `src/color/icc.rs`.
- **EDID auto-configuration** — Reads display's HDR Static Metadata block from EDID. Uses advertised `max_luminance`, `min_luminance`, `max_fall` as defaults when config omits them.
- **DRM compositor handles damage natively** — Same performance as SDR rendering since there's no extra FBO bind or GPU sync.
- **IPC config changes force full redraw** — HDR shader parameter changes (e.g. `sdr_color_intensity`, `gamut_mapping_mode`) are invisible to damage tracking. `reload_output_config` pushes the output to `resized_outputs` AND calls `reset_buffer_ages()` on the DRM compositor to force a full frame render.
- **IPC partial updates merge fields** — When `niri msg output HDMI-A-1 hdr true --gamut-mapping clip` is called, unspecified fields retain their current values. The merge happens in `niri.rs:modify_output_config`.
- **Shader pipeline order matters** — Gamut mapping runs on normalized `[0,1]` values *before* scaling to nits. Running it after scaling causes over-compression because the algorithms see artificially large values.
- **Shader initialization must be idempotent** — `shaders::init()` is called every frame in HDR path. It MUST check if shaders already exist before compiling, otherwise GPU driver leaks memory on each recompilation (~38GB OOM crash observed).
- **Cursor plane artifact (known issue)** — `ALLOW_CURSOR_PLANE_SCANOUT` is disabled in HDR path, but a small transparent square may still appear around the cursor on some GPU/driver combinations. Cursor elements use `Kind::Cursor` which may trigger other scanout paths.

## Critical External Dependency

- **Smithay patch required** — The stackable `tex_program_override` patch lives in `.cargo/git/checkouts/smithay-*/src/backend/renderer/gles/mod.rs`. For CI or other developers, apply `patches/smithay-tex-program-override-stack.patch` before building. Consider forking Smithay for reproducible builds.

## Testing Reminder

Before declaring any phase "complete":
1. Build with `cargo build --release`
2. Test on actual HDR display (verify via display's HDR info overlay)
3. Test with semi-transparent windows (e.g. terminal with opacity) — this was the hardest bug to fix
4. Verify gamma control is disabled in `niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.2`
5. Test IPC partial updates — changing one HDR field should not reset others
6. Test gamut mapping modes with wide-gamut SDR content (e.g. Display P3 images)
7. **NEW:** Test ICC profile produces visible color difference with and without ICC enabled
8. **NEW:** Verify EDID auto-config logs show correct display capabilities on connect
