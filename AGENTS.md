# Niri HDR Development — Project Context

This is the fork `ken-rikard/niri` implementing HDR support for the niri Wayland compositor.

## Repository

- **Origin:** `https://github.com/ken-rikard/niri.git`
- **Upstream:** `https://github.com/YaLTeR/niri.git`

## HDR Feature Branch Structure

| Branch | Role |
|--------|------|
| `feature/hdr-support` | Stable integration branch. Contains only clean, squashed milestones. |
| `feature/hdr-sdr-intensity` | Phase 1 dev branch. Preserves full trial-and-error history (9 commits). |
| `feature/hdr-color-aware` | Phase 2 — per-surface color awareness. |
| `feature/hdr-gamut-mapping` | Phase 4 — gamut mapping. |
| `feature/hdr-icc-profiles` | Phase 3 — ICC profile support. |
| `feature/hdr-dynamic-meta` | Phase 5 — dynamic metadata. |
| `feature/hdr-hlg` | Phase 6 — HLG support. |

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
- `docs/hdr-configuration.md` — User-facing HDR configuration docs
- `src/render_helpers/shaders/hdr_output.frag` — Core HDR shader (PQ EOTF, BT.2020, gamut expansion)
- `src/render_helpers/hdr_output.rs` — Per-element HDR shader override (`HdrWrappedElement`)
- `src/backend/tty.rs` — HDR output pipeline, atomic commit, DRM metadata
- `niri-config/src/output.rs` — HDR config parsing (KDL)
- `niri-ipc/src/lib.rs` — IPC types for runtime HDR control

## Architecture

- **Single-pass per-element rendering** — No offscreen texture. Each element is wrapped with `HdrWrappedElement` which calls `override_default_tex_program()` before drawing.
- **Framebuffer fetch for alpha blending** — `GL_EXT_shader_framebuffer_fetch` decodes PQ framebuffer and blends in linear light. Fallback to premultiplied-PQ path on older GPUs.
- **DRM compositor handles damage natively** — Same performance as SDR rendering since there's no extra FBO bind or GPU sync.
- **IPC config changes force full redraw** — HDR shader parameter changes (e.g. `sdr_color_intensity`) are invisible to damage tracking. `reload_output_config` pushes the output to `resized_outputs` to force a full frame render.

## Testing Reminder

Before declaring any phase "complete":
1. Build with `cargo build --release`
2. Test on actual HDR display (verify via display's HDR info overlay)
3. Test with semi-transparent windows (e.g. terminal with opacity) — this was the hardest bug to fix
4. Verify gamma control is disabled in `niri msg output HDMI-A-1 hdr true --sdr-color-intensity 1.2`
