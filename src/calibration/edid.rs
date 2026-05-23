//! EDID-based display capability detection for HDR auto-configuration.
//!
//! This module extracts HDR capabilities from display EDID data
//! (CTA-861 HDR Static Metadata block) to automatically suggest
//! optimal HDR configuration values.

use libdisplay_info::info::Info;

/// HDR capabilities extracted from a display's EDID.
#[derive(Debug, Clone, Copy)]
pub struct HdrEdidCapabilities {
    /// Display's desired content max luminance (cd/m²).
    /// Maps to `hdr.max_luminance` / `hdr.max_cll`.
    pub max_luminance: f32,
    /// Display's desired content min luminance (cd/m²).
    /// Maps to `hdr.min_luminance`.
    pub min_luminance: f32,
    /// Display's desired max frame-average luminance (cd/m²).
    /// Maps to `hdr.max_fall`.
    pub max_fall: f32,
    /// Whether the display supports PQ (ST 2084) EOTF.
    pub supports_pq: bool,
    /// Whether the display supports HLG (ARIB STD-B67) EOTF.
    pub supports_hlg: bool,
    /// Whether the display supports HDR10 (Type 1 static metadata).
    pub supports_hdr10: bool,
}

impl HdrEdidCapabilities {
    /// Extract HDR capabilities from parsed EDID info.
    ///
    /// Returns `None` if the display does not advertise HDR support
    /// (i.e., no HDR Static Metadata block or max_luminance is 0).
    pub fn from_edid(info: &Info) -> Option<Self> {
        let hdr = info.hdr_static_metadata();

        // If max_luminance is 0, the display doesn't advertise HDR capabilities.
        if hdr.desired_content_max_luminance == 0.0 {
            return None;
        }

        Some(Self {
            max_luminance: hdr.desired_content_max_luminance,
            min_luminance: hdr.desired_content_min_luminance,
            max_fall: hdr.desired_content_max_frame_avg_luminance,
            supports_pq: hdr.pq,
            supports_hlg: hdr.hlg,
            supports_hdr10: hdr.type1,
        })
    }
}

/// Generate a recommended HDR config block (KDL) from EDID capabilities.
pub fn generate_hdr_config(connector: &str, caps: &HdrEdidCapabilities) -> String {
    let tf = if caps.supports_hlg && !caps.supports_pq {
        "hlg"
    } else {
        "pq"
    };

    format!(
        r#"output "{}" {{
    hdr {{
        enabled true
        max-luminance {}
        min-luminance {}
        max-cll {}
        max-fall {}
        transfer-function {}
    }}
}}"#,
        connector,
        caps.max_luminance,
        caps.min_luminance,
        caps.max_luminance,
        caps.max_fall,
        tf
    )
}
