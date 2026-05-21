use std::collections::HashMap;

use smithay::output::Output;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// Supported transfer functions (trafos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferFunction {
    Srgb,
    SrgbLinear,
    Pq,
    Hlg,
    Gamma22,
    Gamma28,
    ExtSrgb,
    Linear,
}

/// Supported color primaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPrimaries {
    Srgb,
    BT2020,
    DisplayP3,
    AdobeRgb,
    ProPhotoRgb,
    Xyz,
    DciP3,
}

/// An image description representing a color space.
#[derive(Debug, Clone)]
pub struct ImageDescription {
    pub transfer_function: TransferFunction,
    pub primaries: ColorPrimaries,
    pub max_luminance: f64,
    pub min_luminance: f64,
    pub max_cll: f64,
    pub max_fall: f64,
    pub mastering_display_primaries: Option<[f64; 8]>,
    pub mastering_white_point: Option<[f64; 2]>,
}

impl Default for ImageDescription {
    fn default() -> Self {
        Self {
            transfer_function: TransferFunction::Srgb,
            primaries: ColorPrimaries::Srgb,
            max_luminance: 1000.0,
            min_luminance: 0.005,
            max_cll: 1000.0,
            max_fall: 400.0,
            mastering_display_primaries: None,
            mastering_white_point: None,
        }
    }
}

pub struct ColorManagementState {
    /// Global ID for the wp_color_manager_v1 global (once created).
    global: Option<GlobalId>,
    /// Per-output color descriptions derived from EDID + config.
    output_image_descriptions: HashMap<Output, ImageDescription>,
    /// Per-surface image descriptions set by clients.
    surface_image_descriptions: HashMap<WlSurface, ImageDescription>,
}

impl ColorManagementState {
    pub fn new() -> Self {
        Self {
            global: None,
            output_image_descriptions: HashMap::new(),
            surface_image_descriptions: HashMap::new(),
        }
    }

    pub fn set_output_global(&mut self, global: GlobalId) {
        self.global = Some(global);
    }

    pub fn set_output_image_description(&mut self, output: &Output, desc: ImageDescription) {
        self.output_image_descriptions.insert(output.clone(), desc);
    }

    pub fn get_output_image_description(&self, output: &Output) -> Option<&ImageDescription> {
        self.output_image_descriptions.get(output)
    }

    pub fn set_surface_image_description(
        &mut self,
        surface: &WlSurface,
        desc: ImageDescription,
    ) {
        self.surface_image_descriptions.insert(surface.clone(), desc);
    }

    pub fn get_surface_image_description(
        &self,
        surface: &WlSurface,
    ) -> Option<&ImageDescription> {
        self.surface_image_descriptions.get(surface)
    }

    pub fn remove_surface_image_description(&mut self, surface: &WlSurface) {
        self.surface_image_descriptions.remove(surface);
    }

    pub fn output_removed(&mut self, output: &Output) {
        self.output_image_descriptions.remove(output);
    }
}

#[macro_export]
macro_rules! delegate_color_management {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        // Placeholder for future protocol delegation.
        // When wp-color-management-v1 protocol bindings are generated,
        // this will delegate GlobalDispatch and Dispatch to ColorManagementState.
        let _ = std::marker::PhantomData::<$ty>;
    };
}

pub trait ColorManagementHandler {
    fn color_management_state(&mut self) -> &mut ColorManagementState;
}

/// Helper to produce an ImageDescription for an HDR output.
pub fn image_description_for_hdr_output(
    max_luminance: f64,
    min_luminance: f64,
    max_cll: f64,
    max_fall: f64,
    is_pq: bool,
    is_hlg: bool,
    is_bt2020: bool,
) -> ImageDescription {
    let transfer_function = if is_pq {
        TransferFunction::Pq
    } else if is_hlg {
        TransferFunction::Hlg
    } else {
        TransferFunction::Srgb
    };
    ImageDescription {
        transfer_function,
        primaries: if is_bt2020 {
            ColorPrimaries::BT2020
        } else {
            ColorPrimaries::Srgb
        },
        max_luminance,
        min_luminance,
        max_cll,
        max_fall,
        mastering_display_primaries: None,
        mastering_white_point: None,
    }
}