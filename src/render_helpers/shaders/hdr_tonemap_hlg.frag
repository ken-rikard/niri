#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D niri_tex;
uniform float u_sdr_brightness_nits;
uniform float u_max_nits;

varying vec2 v_tex_coord;

// HLG inverse OETF: converts HLG signal to linear light.
float hlg_inverse_oetf(float v) {
    const float a = 0.17883277;
    const float b = 0.28466892;
    const float c = 0.55991073;

    if (v <= 0.5) {
        return v * v / 3.0;
    } else {
        return (exp((v - c) / a) + b) / 12.0;
    }
}

// BT.2020 to linear sRGB conversion matrix.
vec3 bt2020_to_linear_srgb(vec3 rgb) {
    mat3 m = mat3(
        1.6605, -0.1246, -0.0182,
        -0.5877, 1.1329, -0.1006,
        -0.0728, -0.0083, 1.1187
    );
    return m * rgb;
}

vec4 hdr_tonemap_hlg(vec4 color) {
    // Apply HLG inverse OETF to get linear light.
    float r = hlg_inverse_oetf(color.r);
    float g = hlg_inverse_oetf(color.g);
    float b = hlg_inverse_oetf(color.b);

    vec3 linear_rgb = vec3(r, g, b);

    // Convert from BT.2020 to linear sRGB for compositing.
    linear_rgb = bt2020_to_linear_srgb(linear_rgb);

    // Apply tone mapping (simple Reinhard).
    float lum = dot(linear_rgb, vec3(0.2126, 0.7152, 0.0722));
    float tone_mapped_lum = lum / (lum + 1.0);
    linear_rgb *= tone_mapped_lum / max(lum, 1e-6);

    // Convert back to sRGB gamma for display.
    vec3 gamma_rgb = pow(clamp(linear_rgb, 0.0, 1.0), vec3(1.0 / 2.2));

    return vec4(gamma_rgb, color.a);
}