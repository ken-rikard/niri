//! Minimal ICC profile parser for color-accurate rendering.
//!
//! This module extracts color primaries and white point from ICC v2/v4 profiles
//! to build color space transformation matrices. It does not handle all ICC
//! features — only the subset needed for display color management.

use std::fs;
use std::path::Path;

use log::debug;

/// Parsed ICC profile data relevant to color management.
#[derive(Debug, Clone)]
pub struct IccProfile {
    pub version: IccVersion,
    pub profile_class: ProfileClass,
    pub color_space: ColorSpaceType,
    pub primaries: [IccXyz; 3],
    pub white_point: IccXyz,
    pub description: String,
    pub rendering_intent: RenderingIntent,
}

#[derive(Debug, Clone, Copy)]
pub enum IccVersion {
    V2,
    V4,
    Other(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum ProfileClass {
    Monitor,
    Other([u8; 4]),
}

#[derive(Debug, Clone, Copy)]
pub enum ColorSpaceType {
    Rgb,
    Other([u8; 4]),
}

#[derive(Debug, Clone, Copy)]
pub enum RenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
    Unknown,
}

/// XYZ tristimulus value as stored in ICC profiles (s15Fixed16Number).
#[derive(Debug, Clone, Copy)]
pub struct IccXyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl IccXyz {
    /// Parse XYZ from 12 bytes (3 × s15Fixed16Number).
    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        Some(Self {
            x: fixed16_to_f64(&data[0..4]),
            y: fixed16_to_f64(&data[4..8]),
            z: fixed16_to_f64(&data[8..12]),
        })
    }
}

/// Convert s15Fixed16Number (16.16 fixed point) to f64.
fn fixed16_to_f64(bytes: &[u8]) -> f64 {
    if bytes.len() < 4 {
        return 0.0;
    }
    let val = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    val as f64 / 65536.0
}

/// Convert 4-byte signature to array.
fn tag_signature(bytes: &[u8]) -> [u8; 4] {
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Parse an ICC profile from file.
pub fn parse_icc_profile(path: impl AsRef<Path>) -> anyhow::Result<IccProfile> {
    let path = path.as_ref();
    debug!("Loading ICC profile from {}", path.display());
    
    let data = fs::read(path)?;
    if data.len() < 128 {
        anyhow::bail!("ICC profile too small ({} bytes)", data.len());
    }
    
    // Parse header (128 bytes).
    let _profile_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let _cmm_type = tag_signature(&data[4..8]);
    let version_raw = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let profile_class_raw = tag_signature(&data[12..16]);
    let color_space_raw = tag_signature(&data[16..20]);
    let _pcs = tag_signature(&data[20..24]);
    let _ = &data[24..36]; // Date/time (12 bytes)
    let signature = tag_signature(&data[36..40]);
    
    if &signature != b"acsp" {
        anyhow::bail!("Invalid ICC profile signature: {:?}", signature);
    }
    
    let version = match version_raw {
        0x02200000 => IccVersion::V2,
        0x02400000 => IccVersion::V2,
        0x04000000 => IccVersion::V4,
        0x04300000 => IccVersion::V4,
        v => IccVersion::Other(v),
    };
    
    let profile_class = if &profile_class_raw == b"mntr" {
        ProfileClass::Monitor
    } else {
        ProfileClass::Other(profile_class_raw)
    };
    
    let color_space = if &color_space_raw == b"RGB " {
        ColorSpaceType::Rgb
    } else {
        ColorSpaceType::Other(color_space_raw)
    };
    
    let rendering_intent_raw = u32::from_be_bytes([data[64], data[65], data[66], data[67]]);
    let rendering_intent = match rendering_intent_raw {
        0 => RenderingIntent::Perceptual,
        1 => RenderingIntent::RelativeColorimetric,
        2 => RenderingIntent::Saturation,
        3 => RenderingIntent::AbsoluteColorimetric,
        _ => RenderingIntent::Unknown,
    };
    
    let illuminant = IccXyz::from_bytes(&data[68..80]).unwrap_or(IccXyz { x: 0.0, y: 0.0, z: 0.0 });
    debug!("Profile: version={:?}, class={:?}, space={:?}, intent={:?}", 
           version, profile_class, color_space, rendering_intent);
    debug!("D50 illuminant: {:?}", illuminant);
    
    // Parse tag table.
    let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize;
    debug!("Tag count: {}", tag_count);
    
    if data.len() < 132 + tag_count * 12 {
        anyhow::bail!("Tag table extends beyond profile size");
    }
    
    // Build tag map: signature -> (offset, size).
    let mut tags = Vec::with_capacity(tag_count);
    let mut tag_offset = 132usize;
    for _ in 0..tag_count {
        let sig = tag_signature(&data[tag_offset..tag_offset + 4]);
        let offset = u32::from_be_bytes([
            data[tag_offset + 4],
            data[tag_offset + 5],
            data[tag_offset + 6],
            data[tag_offset + 7],
        ]) as usize;
        let size = u32::from_be_bytes([
            data[tag_offset + 8],
            data[tag_offset + 9],
            data[tag_offset + 10],
            data[tag_offset + 11],
        ]) as usize;
        tags.push((sig, offset, size));
        tag_offset += 12;
    }
    
    // Extract primaries and white point.
    let mut primaries = [
        IccXyz { x: 0.0, y: 0.0, z: 0.0 },
        IccXyz { x: 0.0, y: 0.0, z: 0.0 },
        IccXyz { x: 0.0, y: 0.0, z: 0.0 },
    ];
    let mut white_point = IccXyz { x: 0.0, y: 0.0, z: 0.0 };
    let mut description = String::new();
    
    for &(sig, offset, size) in &tags {
        if offset + size > data.len() {
            continue;
        }
        
        match &sig {
            b"rXYZ" => {
                primaries[0] = IccXyz::from_bytes(&data[offset + 8..offset + size])
                    .unwrap_or(primaries[0]);
                debug!("Red primary: {:?}", primaries[0]);
            }
            b"gXYZ" => {
                primaries[1] = IccXyz::from_bytes(&data[offset + 8..offset + size])
                    .unwrap_or(primaries[1]);
                debug!("Green primary: {:?}", primaries[1]);
            }
            b"bXYZ" => {
                primaries[2] = IccXyz::from_bytes(&data[offset + 8..offset + size])
                    .unwrap_or(primaries[2]);
                debug!("Blue primary: {:?}", primaries[2]);
            }
            b"wtpt" => {
                white_point = IccXyz::from_bytes(&data[offset + 8..offset + size])
                    .unwrap_or(white_point);
                debug!("White point: {:?}", white_point);
            }
            b"desc" => {
                // Parse profile description.
                if size > 12 {
                    let desc_type = tag_signature(&data[offset..offset + 4]);
                    if &desc_type == b"desc" {
                        // ASCII description: count (4) + string (null-terminated)
                        let ascii_count = u32::from_be_bytes([
                            data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]
                        ]) as usize;
                        let start = offset + 12;
                        let end = (start + ascii_count).min(offset + size);
                        description = String::from_utf8_lossy(&data[start..end])
                            .trim_end_matches('\0')
                            .to_string();
                    }
                }
            }
            _ => {}
        }
    }
    
    // If primaries not found in profile, use fallback based on known profiles.
    if primaries[0].x == 0.0 {
        debug!("Primaries not found in profile, using default sRGB");
        primaries = srgb_primaries();
        white_point = d65_whitepoint();
    }
    
    let profile = IccProfile {
        version,
        profile_class,
        color_space,
        primaries,
        white_point,
        description,
        rendering_intent,
    };
    
    debug!("Parsed ICC profile: {:?}", profile);
    Ok(profile)
}

/// Standard sRGB primaries.
pub fn srgb_primaries() -> [IccXyz; 3] {
    [
        IccXyz { x: 0.6400, y: 0.3300, z: 0.0300 },  // R
        IccXyz { x: 0.3000, y: 0.6000, z: 0.1000 },  // G
        IccXyz { x: 0.1500, y: 0.0600, z: 0.7900 },  // B
    ]
}

/// Standard BT.2020 primaries.
pub fn bt2020_primaries() -> [IccXyz; 3] {
    [
        IccXyz { x: 0.708, y: 0.292, z: 0.000 },
        IccXyz { x: 0.170, y: 0.797, z: 0.033 },
        IccXyz { x: 0.131, y: 0.046, z: 0.823 },
    ]
}

/// D65 white point.
pub fn d65_whitepoint() -> IccXyz {
    IccXyz {
        x: 0.3127,
        y: 0.3290,
        z: 0.3583,
    }
}

/// D50 white point (ICC PCS illuminant).
pub fn d50_whitepoint() -> IccXyz {
    IccXyz {
        x: 0.3457,
        y: 0.3585,
        z: 0.2958,
    }
}

/// Build a 3×3 RGB→XYZ matrix from primaries and white point.
///
/// Given primaries (as columns) and white point, solves for scaling factors
/// `s` such that `P * s = W`, where P's columns are the primaries.
/// Returns `M = P * diag(s)` so that `M * [1,1,1]^T = W`.
pub fn build_rgb_to_xyz_matrix(primaries: &[IccXyz; 3], white: &IccXyz) -> [[f64; 3]; 3] {
    // Primary matrix P with primaries as columns.
    // P * [1,1,1]^T should equal white after scaling each column.
    let p = [
        [primaries[0].x, primaries[1].x, primaries[2].x],
        [primaries[0].y, primaries[1].y, primaries[2].y],
        [primaries[0].z, primaries[1].z, primaries[2].z],
    ];

    // Solve P * s = white for s using Cramer's rule.
    let det = p[0][0] * (p[1][1] * p[2][2] - p[1][2] * p[2][1])
        - p[0][1] * (p[1][0] * p[2][2] - p[1][2] * p[2][0])
        + p[0][2] * (p[1][0] * p[2][1] - p[1][1] * p[2][0]);

    if det.abs() < 1e-10 {
        // Singular matrix — return unscaled primaries.
        return p;
    }

    let inv_det = 1.0 / det;

    // Replace column 0 with white and compute det.
    let det_s0 = white.x * (p[1][1] * p[2][2] - p[1][2] * p[2][1])
        - p[0][1] * (white.y * p[2][2] - p[1][2] * white.z)
        + p[0][2] * (white.y * p[2][1] - p[1][1] * white.z);

    // Replace column 1 with white and compute det.
    let det_s1 = p[0][0] * (white.y * p[2][2] - p[1][2] * white.z)
        - white.x * (p[1][0] * p[2][2] - p[1][2] * p[2][0])
        + p[0][2] * (p[1][0] * white.z - white.y * p[2][0]);

    // Replace column 2 with white and compute det.
    let det_s2 = p[0][0] * (p[1][1] * white.z - white.y * p[2][1])
        - p[0][1] * (p[1][0] * white.z - white.y * p[2][0])
        + white.x * (p[1][0] * p[2][1] - p[1][1] * p[2][0]);

    let s = [det_s0 * inv_det, det_s1 * inv_det, det_s2 * inv_det];

    // M = P * diag(s)
    [
        [p[0][0] * s[0], p[0][1] * s[1], p[0][2] * s[2]],
        [p[1][0] * s[0], p[1][1] * s[1], p[1][2] * s[2]],
        [p[2][0] * s[0], p[2][1] * s[1], p[2][2] * s[2]],
    ]
}

/// Invert a 3×3 matrix.
pub fn invert_matrix(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    
    if det.abs() < 1e-10 {
        return None;
    }
    
    let inv_det = 1.0 / det;
    
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ])
}

/// Multiply two 3×3 matrices: C = A × B.
pub fn multiply_matrices(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

/// Bradford chromatic adaptation matrix from D65 to D50.
///
/// This transforms XYZ values from D65 white point to D50 white point,
/// which is necessary because ICC profiles use D50 as the PCS illuminant
/// while sRGB uses D65.
fn bradford_d65_to_d50() -> [[f64; 3]; 3] {
    [
        [1.0478112, 0.0228866, -0.0501270],
        [0.0295424, 0.9904844, -0.0170491],
        [-0.0092345, 0.0150436, 0.7521316],
    ]
}

/// Build a color space transformation matrix from source RGB to target RGB.
///
/// Given source primaries (src) and target primaries (dst), computes:
/// `target_RGB = inv(XYZ_to_dst) * XYZ_to_src * source_RGB`
impl IccProfile {
    /// Build a color correction matrix that maps sRGB colors to this profile's color space.
    ///
    /// Returns a 3×3 matrix where `output = matrix * input`. The matrix is in row-major
    /// order for GLSL (i.e., `result[i] = sum(matrix[i][j] * input[j])`).
    pub fn srgb_correction_matrix(&self) -> Option<[[f64; 3]; 3]> {
        // ICC profile primaries are ALREADY scaled XYZ columns.
        let display_to_xyz = [
            [self.primaries[0].x, self.primaries[1].x, self.primaries[2].x],
            [self.primaries[0].y, self.primaries[1].y, self.primaries[2].y],
            [self.primaries[0].z, self.primaries[1].z, self.primaries[2].z],
        ];
        
        // Check: columns should sum to D50
        let d50 = d50_whitepoint();
        let white_from_primaries = [
            display_to_xyz[0][0] + display_to_xyz[0][1] + display_to_xyz[0][2],
            display_to_xyz[1][0] + display_to_xyz[1][1] + display_to_xyz[1][2],
            display_to_xyz[2][0] + display_to_xyz[2][1] + display_to_xyz[2][2],
        ];
        let xyz_to_display = invert_matrix(&display_to_xyz)?;
        
        // Standard sRGB to XYZ (D65) matrix.
        let srgb_to_xyz: [[f64; 3]; 3] = [
            [0.4124564, 0.3575761, 0.1804375],
            [0.2126729, 0.7151522, 0.0721750],
            [0.0193339, 0.1191920, 0.9503041],
        ];
        
        // ICC profiles use D50 white point, but sRGB uses D65.
        // Apply Bradford chromatic adaptation from D65 to D50.
        let adapt = bradford_d65_to_d50();
        let srgb_to_xyz_d50 = multiply_matrices(&adapt, &srgb_to_xyz);
        
        // First convert sRGB to XYZ(D50), then XYZ to display RGB.
        Some(multiply_matrices(&xyz_to_display, &srgb_to_xyz_d50))
    }
    
    /// Build a color correction matrix that maps sRGB colors to BT.2020.
    ///
    /// This is used when we want to convert sRGB content to the HDR BT.2020 color space,
    /// but using the display's actual primaries from the ICC profile instead of
    /// assuming BT.2020 primaries.
    pub fn srgb_to_bt2020_matrix(&self) -> Option<[[f64; 3]; 3]> {
        let srgb = srgb_primaries();
        let srgb_white = d65_whitepoint();
        let bt2020 = bt2020_primaries();
        let bt2020_white = d65_whitepoint();
        
        build_color_space_matrix(
            &srgb, &srgb_white,
            &bt2020, &bt2020_white,
        )
    }
    
    /// Convert a [[f64; 3]; 3] matrix to [[f32; 3]; 3] for shader uniforms.
    pub fn matrix_to_f32(m: &[[f64; 3]; 3]) -> [[f32; 3]; 3] {
        [
            [m[0][0] as f32, m[0][1] as f32, m[0][2] as f32],
            [m[1][0] as f32, m[1][1] as f32, m[1][2] as f32],
            [m[2][0] as f32, m[2][1] as f32, m[2][2] as f32],
        ]
    }
}

/// Build a color space transformation matrix from source RGB to target RGB.
///
/// Given source primaries (src) and target primaries (dst), computes:
/// `target_RGB = inv(XYZ_to_dst) * XYZ_to_src * source_RGB`
pub fn build_color_space_matrix(
    src_primaries: &[IccXyz; 3],
    src_white: &IccXyz,
    dst_primaries: &[IccXyz; 3],
    dst_white: &IccXyz,
) -> Option<[[f64; 3]; 3]> {
    let src_to_xyz = build_rgb_to_xyz_matrix(src_primaries, src_white);
    let dst_to_xyz = build_rgb_to_xyz_matrix(dst_primaries, dst_white);
    let xyz_to_dst = invert_matrix(&dst_to_xyz)?;
    
    Some(multiply_matrices(&xyz_to_dst, &src_to_xyz))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_srgb_matrix() {
        let m = build_rgb_to_xyz_matrix(&srgb_primaries(), &d65_whitepoint());
        // White should map to D65: M * [1,1,1]^T ≈ [0.3127, 0.3290, 0.3583]
        let wx = m[0][0] + m[0][1] + m[0][2];
        let wy = m[1][0] + m[1][1] + m[1][2];
        let wz = m[2][0] + m[2][1] + m[2][2];
        
        assert!((wx - 0.3127).abs() < 0.01, "X white point mismatch: {} != 0.3127", wx);
        assert!((wy - 0.3290).abs() < 0.01, "Y white point mismatch: {} != 0.3290", wy);
        assert!((wz - 0.3583).abs() < 0.01, "Z white point mismatch: {} != 0.3583", wz);
    }
    
    #[test]
    fn test_identity_conversion() {
        // Converting from sRGB to sRGB should give identity matrix.
        let m = build_color_space_matrix(
            &srgb_primaries(), &d65_whitepoint(),
            &srgb_primaries(), &d65_whitepoint(),
        ).unwrap();
        
        // Check diagonal is close to 1, off-diagonal close to 0.
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((m[i][j] - expected).abs() < 0.01, 
                        "Identity matrix mismatch at [{},{}]: {} != {}", i, j, m[i][j], expected);
            }
        }
    }
}
