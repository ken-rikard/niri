use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, DamageSnapshot, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Rectangle, Scale, Transform};

use super::renderer::AsGlesFrame as _;
use super::shaders::Shaders;
use super::texture::TextureRenderElement;
use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::niri::OutputRenderElements;

/// Which HDR treatment to apply to an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdrTreatment {
    /// Apply SDR->HDR conversion (sRGB to PQ, gamut expansion, etc.)
    Convert,
    /// Pass content through as-is (content is already HDR-native)
    Passthrough,
}

/// Wraps an OutputRenderElements to apply the HDR shader when drawn.
/// This eliminates the offscreen texture by applying tone mapping per-element
/// directly during the DRM compositor's single render pass.
#[derive(Debug)]
pub struct HdrWrappedElement<'a> {
    inner: &'a OutputRenderElements<TtyRenderer<'a>>,
    conversion_program: GlesTexProgram,
    passthrough_program: GlesTexProgram,
    treatment: HdrTreatment,
    sdr_brightness_nits: f32,
    max_nits: f32,
    sdr_color_intensity: f32,
    gamut_mapping_mode: i32,
    transfer_function: i32,
}

impl<'a> HdrWrappedElement<'a> {
    pub fn new(
        inner: &'a OutputRenderElements<TtyRenderer<'a>>,
        conversion_program: GlesTexProgram,
        passthrough_program: GlesTexProgram,
        treatment: HdrTreatment,
        sdr_brightness_nits: f32,
        max_nits: f32,
        sdr_color_intensity: f32,
        gamut_mapping_mode: i32,
        transfer_function: i32,
    ) -> Self {
        Self {
            inner,
            conversion_program,
            passthrough_program,
            treatment,
            sdr_brightness_nits,
            max_nits,
            sdr_color_intensity,
            gamut_mapping_mode,
            transfer_function,
        }
    }

    fn conversion_uniforms(&self) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_sdr_brightness_nits", self.sdr_brightness_nits),
            Uniform::new("u_max_nits", self.max_nits),
            Uniform::new("u_sdr_color_intensity", self.sdr_color_intensity),
            Uniform::new("u_gamut_mapping_mode", self.gamut_mapping_mode),
            Uniform::new("u_transfer_function", self.transfer_function),
        ]
    }
}

impl Element for HdrWrappedElement<'_> {
    fn id(&self) -> &Id {
        self.inner.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }

    fn transform(&self) -> Transform {
        self.inner.transform()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.inner.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }

    fn kind(&self) -> Kind {
        self.inner.kind()
    }
}

impl<'render> RenderElement<TtyRenderer<'render>> for HdrWrappedElement<'render> {
    fn draw(
        &self,
        frame: &mut TtyFrame<'render, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), TtyRendererError<'render>> {
        let (program, uniforms, _treatment_name) = match self.treatment {
            HdrTreatment::Convert => (
                self.conversion_program.clone(),
                self.conversion_uniforms(),
                "convert",
            ),
            HdrTreatment::Passthrough => (
                self.passthrough_program.clone(),
                vec![],
                "passthrough",
            ),
        };

        // Set shader override before drawing.
        frame
            .as_gles_frame()
            .override_default_tex_program(program, uniforms);

        // Draw the inner element (its TtyRenderer draw will use the overridden shader).
        let result = RenderElement::<TtyRenderer<'render>>::draw(
            self.inner,
            frame,
            src,
            dst,
            damage,
            opaque_regions,
            cache,
        );

        // Clear the override.
        frame.as_gles_frame().clear_tex_program_override();

        result
    }

    fn underlying_storage(
        &self,
        _renderer: &mut TtyRenderer<'render>,
    ) -> Option<UnderlyingStorage<'_>> {
        // Return None to prevent DRM direct scanout.
        // The DRM compositor must call draw() so our HDR shader override
        // is applied. Without this, it would bypass our shader entirely
        // and display raw SRGB buffers directly on the HDR output.
        None
    }
}

// Keep the old HdrOutputRenderElement for potential fallback use.
#[derive(Debug, Clone)]
pub struct HdrOutputRenderElement {
    inner: TextureRenderElement<GlesTexture>,
    program: Option<GlesTexProgram>,
    sdr_brightness_nits: f32,
    max_nits: f32,
    sdr_color_intensity: f32,
    /// Accumulated damage history from the offscreen render passes.
    damage: DamageSnapshot<i32, Physical>,
}

impl HdrOutputRenderElement {
    pub fn new(
        texture: TextureRenderElement<GlesTexture>,
        program: Option<GlesTexProgram>,
        sdr_brightness_nits: f32,
        max_nits: f32,
        sdr_color_intensity: f32,
        damage: DamageSnapshot<i32, Physical>,
    ) -> Self {
        Self {
            inner: texture,
            program,
            sdr_brightness_nits,
            max_nits,
            sdr_color_intensity,
            damage,
        }
    }

    pub fn from_frame(
        frame: &mut GlesFrame<'_, '_>,
        inner: TextureRenderElement<GlesTexture>,
        sdr_brightness_nits: f32,
        max_nits: f32,
        sdr_color_intensity: f32,
    ) -> Option<Self> {
        let program = Shaders::get_from_frame(frame).hdr_output.clone();
        if program.is_none() {
            return None;
        }
        Some(Self {
            inner,
            program,
            sdr_brightness_nits,
            max_nits,
            sdr_color_intensity,
            damage: DamageSnapshot::empty(),
        })
    }
}

impl Element for HdrOutputRenderElement {
    fn id(&self) -> &Id {
        self.inner.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.damage.current_commit()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }

    fn transform(&self) -> Transform {
        self.inner.transform()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.damage
            .damage_since(commit)
            .unwrap_or_else(|| DamageSet::from_slice(&[self.geometry(scale)]))
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.inner.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl RenderElement<GlesRenderer> for HdrOutputRenderElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        let Some(program) = &self.program else {
            return Ok(());
        };

        let uniforms = vec![
            Uniform::new("u_sdr_brightness_nits", self.sdr_brightness_nits),
            Uniform::new("u_max_nits", self.max_nits),
            Uniform::new("u_sdr_color_intensity", self.sdr_color_intensity),
        ];

        frame.override_default_tex_program(program.clone(), uniforms);
        let result = RenderElement::<GlesRenderer>::draw(
            &self.inner,
            frame,
            src,
            dst,
            damage,
            opaque_regions,
            cache,
        );
        frame.clear_tex_program_override();
        result
    }

    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        None
    }
}

impl<'render> RenderElement<TtyRenderer<'render>> for HdrOutputRenderElement {
    fn draw(
        &self,
        frame: &mut TtyFrame<'_, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), TtyRendererError<'render>> {
        let gles_frame = frame.as_gles_frame();
        RenderElement::<GlesRenderer>::draw(
            self,
            gles_frame,
            src,
            dst,
            damage,
            opaque_regions,
            cache,
        )?;
        Ok(())
    }

    fn underlying_storage(
        &self,
        _renderer: &mut TtyRenderer<'render>,
    ) -> Option<UnderlyingStorage<'_>> {
        None
    }
}