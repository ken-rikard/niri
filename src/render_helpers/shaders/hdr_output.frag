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
vec3 expand_gamut(vec3 linear_rgb, float intensity) {
    float luminance = dot(linear_rgb, vec3(0.2126, 0.7152, 0.0722));
    vec3 chroma = linear_rgb - luminance;
    return luminance + chroma * intensity;
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

    // Convert source sRGB to linear BT.2020 nits.
    vec3 src_linear = vec3(
        srgb_to_linear(src_srgb.r),
        srgb_to_linear(src_srgb.g),
        srgb_to_linear(src_srgb.b)
    );
    src_linear = expand_gamut(src_linear, u_sdr_color_intensity);
    src_linear *= u_sdr_brightness_nits;
    src_linear = srgb_to_bt2020(src_linear);

#ifdef HAS_FRAMEBUFFER_FETCH
    // Read current framebuffer value (PQ encoded) and decode to linear nits.
    vec4 fb = gl_LastFragColor;
    vec3 fb_linear = vec3(
        pq_eotf(fb.r),
        pq_eotf(fb.g),
        pq_eotf(fb.b)
    );

    // Blend in linear light space (physically correct).
    vec3 result_linear = src_linear * src_alpha + fb_linear * (1.0 - src_alpha);

    // Encode back to PQ.
    vec3 result_pq = vec3(
        pq_oetf(result_linear.r),
        pq_oetf(result_linear.g),
        pq_oetf(result_linear.b)
    );

    // Output with alpha=1.0 so GL blending becomes a no-op.
    gl_FragColor = vec4(result_pq, 1.0);
#else
    // Fallback: simple PQ output (blending in PQ space - incorrect for
    // semi-transparent elements but works for opaque content).
    vec3 pq = vec3(
        pq_oetf(src_linear.r),
        pq_oetf(src_linear.g),
        pq_oetf(src_linear.b)
    );
    gl_FragColor = vec4(pq * src_alpha, src_alpha);
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        gl_FragColor = vec4(0.0, 0.2, 0.0, 0.2) + gl_FragColor * 0.8;
#endif
}