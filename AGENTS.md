# Niri HDR Development — Project Context

This is the fork `ken-rikard/niri` implementing HDR support for the niri Wayland compositor.

## Repository

- **Origin:** `https://github.com/ken-rikard/niri.git`
- **Upstream:** `https://github.com/YaLTeR/niri.git`

## HDR Feature Branch Structure

| Branch | Role |
|--------|------|
| `feature/hdr-support` | Stable integration branch. Contains only clean, squashed milestones. Currently at **Phase 3 (ICC) + Phase 10A (EDID auto-config)**. |
| `feature/hdr-sdr-intensity` | Phase 1 dev branch. Preserves full trial-and-error history (9 commits). |
| `feature/hdr-color-aware` | Phase 2 — per-surface color awareness. Per-element passthrough implemented. ⚠️ **NOT YET TESTED** on real HDR display. |
| `feature/hdr-gamut-mapping` | Phase 4 — gamut mapping. ✅ Merged into hdr-support. Preserves full dev history. |
| `feature/hdr-dynamic-meta` | Phase 5 — dynamic metadata. ✅ Merged into hdr-support. |
| `feature/hdr-hlg` | Phase 6 — HLG support. ✅ Merged into hdr-support. Preserves full dev history. |
| `feature/hdr-icc-profiles` | Phase 3 — ICC profile support. ✅ Merged into hdr-support. Preserves full dev history. |
| `feature/hdr-calibration-wizard` | Phase 10 — HDR calibration wizard. 🚧 **IN PROGRESS** (EDID auto-config 10.2 implemented). |

## Required Git Workflow

**When integrating a completed phase into `feature/hdr-support`:**

1. Use `--squash` merge. The stable branch must NEVER contain fixup/debug/refactor commits.
2. Keep the original feature branch intact (preserves full dev history).
3. After squashing into `hdr-support`, rebase all dependent phase branches onto the new HEAD.
4. Force-push `hdr-support` and the rebased branches when necessary.

```bash
# Integrate a completed phase
git checkout feature/hdr-support
git merge --squash feature/hdr-PHASE
git commit -m "feat(HDR): Phase N — description"
git push --force-with-lease origin feature/hdr-support

# Rebase dependent branches
for b in feature/hdr-color-aware feature/hdr-gamut-mapping ...; do
  git checkout $b
  git rebase feature/hdr-support
  git push --force-with-lease origin $b
done
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
