use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Rectangle, Scale, Transform};

use super::renderer::AsGlesFrame as _;
use super::shaders::Shaders;
use super::texture::TextureRenderElement;
use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};

#[derive(Debug, Clone)]
pub struct HdrOutputRenderElement {
    inner: TextureRenderElement<GlesTexture>,
    program: Option<GlesTexProgram>,
    sdr_brightness_nits: f32,
    max_nits: f32,
}

impl HdrOutputRenderElement {
    pub fn new(
        texture: TextureRenderElement<GlesTexture>,
        program: Option<GlesTexProgram>,
        sdr_brightness_nits: f32,
        max_nits: f32,
    ) -> Self {
        Self {
            inner: texture,
            program,
            sdr_brightness_nits,
            max_nits,
        }
    }

    pub fn from_frame(
        frame: &mut GlesFrame<'_, '_>,
        inner: TextureRenderElement<GlesTexture>,
        sdr_brightness_nits: f32,
        max_nits: f32,
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
        })
    }
}

impl Element for HdrOutputRenderElement {
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
        ];
        frame.override_default_tex_program(program.clone(), uniforms);
        RenderElement::<GlesRenderer>::draw(
            &self.inner,
            frame,
            src,
            dst,
            damage,
            opaque_regions,
            cache,
        )?;
        frame.clear_tex_program_override();
        Ok(())
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
            &self,
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
