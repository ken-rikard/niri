//_DEFINES_

// Enable framebuffer fetch for correct alpha blending in PQ space.
// Without this, semi-transparent overlays cause drastic darkening because
// GL blending interpolates PQ-encoded values (nonlinear) instead of linear light.
#ifdef GL_EXT_shader_framebuffer_fetch
#extension GL_EXT_shader_framebuffer_fetch : enable
#define HAS_FRAMEBUFFER_FETCH
#endif

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D tex;
uniform float alpha;
uniform float u_sdr_brightness_nits;
uniform float u_max_nits;
uniform float u_sdr_color_intensity;
uniform int u_gamut_mapping_mode;  // 0=desaturate, 1=clip, 2=relative
uniform int u_transfer_function;  // 0=PQ (ST 2084), 1=HLG (ARIB STD-B67)
uniform int u_icc_enabled;  // 0=disabled, 1=enabled
uniform mat3 u_icc_matrix;  // sRGB→ICC display color space matrix

// Corner clipping uniforms (ignored when geo_size.x == 0)
uniform float niri_scale;
uniform vec2 geo_size;
uniform vec4 corner_radius;
uniform mat3 input_to_geo;

float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

varying vec2 v_coords;

// sRGB to linear conversion.
float srgb_to_linear(float c) {
    if (c <= 0.04045) {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

// PQ (ST 2084) constants.
const float PQ_m1 = 0.1593017578125;
const float PQ_m2 = 78.84375;
const float PQ_c1 = 0.8359375;
const float PQ_c2 = 18.8515625;
const float PQ_c3 = 18.6875;

// PQ OETF: converts linear light in nits to PQ-encoded signal.
float pq_oetf(float linear_nits) {
    float l = linear_nits / 10000.0;
    float l_m1 = pow(max(l, 1e-6), PQ_m1);
    float num = PQ_c1 + PQ_c2 * l_m1;
    float den = 1.0 + PQ_c3 * l_m1;
    return pow(num / max(den, 1e-6), PQ_m2);
}

// PQ EOTF: decodes PQ signal back to linear light in nits.
float pq_eotf(float pq) {
    float p = pow(max(pq, 0.0), 1.0 / PQ_m2);
    float num = max(p - PQ_c1, 0.0);
    float den = PQ_c2 - PQ_c3 * p;
    return pow(num / max(den, 1e-6), 1.0 / PQ_m1) * 10000.0;
}

// HLG (ARIB STD-B67 / ITU-R BT.2100) constants.
// Reference white is 1000 nits (same as PQ).
const float HLG_a = 0.17883277;
const float HLG_b = 0.28466892;
const float HLG_c = 0.55991073;

// HLG OETF: converts linear light in nits to HLG-encoded signal.
float hlg_oetf(float linear_nits) {
    float e = linear_nits / 10000.0;
    if (e <= 1.0 / 12.0) {
        return sqrt(3.0 * e);
    } else {
        return HLG_a * log(12.0 * e - HLG_b) + HLG_c;
    }
}

// HLG EOTF: decodes HLG signal back to linear light in nits.
float hlg_eotf(float hlg) {
    if (hlg <= 0.5) {
        float e = (hlg * hlg) / 3.0;
        return e * 10000.0;
    } else {
        float e = (exp((hlg - HLG_c) / HLG_a) + HLG_b) / 12.0;
        return e * 10000.0;
    }
}

// Convert linear sRGB to linear BT.2020.
vec3 srgb_to_bt2020(vec3 rgb) {
    // Column-major: each column is (R_coeff, G_coeff, B_coeff) for that input channel.
    mat3 m = mat3(
        0.6274, 0.0691, 0.0164,  // column 0: how much R_in contributes to R/G/B out
        0.3293, 0.9196, 0.0880,  // column 1: how much G_in contributes to R/G/B out
        0.0433, 0.0113, 0.8956   // column 2: how much B_in contributes to R/G/B out
    );
    return m * rgb;
}

// Gamut expansion: scales chroma in linear space.
// At intensity=1.0, colors pass through unchanged.
// At intensity>1.0, chroma is amplified for more vibrant colors.
vec3 expand_gamut(vec3 linear_rgb, float intensity) {
    float luminance = dot(linear_rgb, vec3(0.2126, 0.7152, 0.0722));
    vec3 chroma = linear_rgb - luminance;
    return luminance + chroma * intensity;
}

// Gamut mapping: handles out-of-gamut colors after BT.2020 conversion.
vec3 gamut_map(vec3 bt2020, int mode) {
    if (mode == 0) {
        // Desaturate: reduce saturation for out-of-gamut colors (KWin default).
        float lum = dot(bt2020, vec3(0.2627, 0.6780, 0.0593));
        vec3 chroma = bt2020 - lum;
        float max_chroma = max(max(abs(chroma.r), abs(chroma.g)), abs(chroma.b));
        if (max_chroma > 0.5) {
            float scale = 0.5 / max_chroma;
            return lum + chroma * scale;
        }
        return bt2020;
    } else if (mode == 1) {
        // Clip: simple clamping to [0, 1].
        return clamp(bt2020, 0.0, 1.0);
    } else if (mode == 2) {
        // Relative: preserve color relationships while compressing gamut.
        // Scale all channels proportionally if any channel exceeds 1.0.
        float max_channel = max(max(bt2020.r, bt2020.g), bt2020.b);
        if (max_channel > 1.0) {
            return bt2020 / max_channel;
        }
        return bt2020;
    }
    return bt2020;
}

void main() {
    // Sample input texture (premultiplied sRGB gamma encoded).
    vec4 texel = texture2D(tex, v_coords);

    // Compute effective alpha (pixel alpha * element alpha).
#if defined(NO_ALPHA)
    float src_alpha = alpha;
    vec3 src_srgb = texel.rgb;
#else
    float src_alpha = texel.a * alpha;
    // Un-premultiply to get straight sRGB color.
    vec3 src_srgb = (texel.a > 0.001) ? texel.rgb / texel.a : vec3(0.0);
#endif

    // Apply corner clipping when geo_size is set (ClippedSurfaceRenderElement).
    if (geo_size.x > 0.0) {
        vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
        if (coords_geo.x < 0.0 || 1.0 < coords_geo.x || coords_geo.y < 0.0 || 1.0 < coords_geo.y) {
            src_alpha = 0.0;
        } else {
            src_alpha *= niri_rounding_alpha(coords_geo.xy * geo_size, geo_size, corner_radius);
        }
    }

    // Convert source sRGB to linear BT.2020 (normalized [0,1]).
    vec3 src_linear = vec3(
        srgb_to_linear(src_srgb.r),
        srgb_to_linear(src_srgb.g),
        srgb_to_linear(src_srgb.b)
    );
    src_linear = expand_gamut(src_linear, u_sdr_color_intensity);
    
    // Apply ICC profile color correction when available, otherwise use BT.2020.
    if (u_icc_enabled == 1) {
        src_linear = u_icc_matrix * src_linear;
    } else {
        src_linear = srgb_to_bt2020(src_linear);
    }
    
    src_linear = gamut_map(src_linear, u_gamut_mapping_mode);

    // Scale to nits AFTER gamut mapping (which expects [0,1] values).
    src_linear *= u_sdr_brightness_nits;

#ifdef HAS_FRAMEBUFFER_FETCH
    // Decode the existing framebuffer and blend in linear light.
    vec3 fb_encoded = gl_LastFragColor.rgb;
    float fb_alpha = gl_LastFragColor.a;

    // Decode framebuffer to linear nits (use same transfer function).
    vec3 fb_linear;
    if (u_transfer_function == 1) {
        fb_linear = vec3(hlg_eotf(fb_encoded.r), hlg_eotf(fb_encoded.g), hlg_eotf(fb_encoded.b));
    } else {
        fb_linear = vec3(pq_eotf(fb_encoded.r), pq_eotf(fb_encoded.g), pq_eotf(fb_encoded.b));
    }

    // Alpha blend in linear light.
    vec3 blended = fb_linear * (1.0 - src_alpha) + src_linear * src_alpha;

    // Encode back to output transfer function.
    vec3 out_encoded;
    if (u_transfer_function == 1) {
        out_encoded = vec3(hlg_oetf(blended.r), hlg_oetf(blended.g), hlg_oetf(blended.b));
    } else {
        out_encoded = vec3(pq_oetf(blended.r), pq_oetf(blended.g), pq_oetf(blended.b));
    }

    gl_FragColor = vec4(out_encoded, max(fb_alpha, src_alpha));
#else
    // Fallback: simple output (blending in encoded space - incorrect for
    // semi-transparent elements but works for opaque content).
    vec3 encoded;
    if (u_transfer_function == 1) {
        encoded = vec3(hlg_oetf(src_linear.r), hlg_oetf(src_linear.g), hlg_oetf(src_linear.b));
    } else {
        encoded = vec3(pq_oetf(src_linear.r), pq_oetf(src_linear.g), pq_oetf(src_linear.b));
    }

    gl_FragColor = vec4(encoded, src_alpha);
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        gl_FragColor = vec4(0.0, 0.2, 0.0, 0.2) + gl_FragColor * 0.8;
#endif
}