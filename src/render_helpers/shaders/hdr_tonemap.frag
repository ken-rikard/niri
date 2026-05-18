#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D niri_tex;
uniform float u_sdr_brightness_nits;
uniform float u_max_nits;
uniform float u_contrast;

varying vec2 v_tex_coord;

// PQ (ST 2084) EOTF: converts normalized PQ signal to linear light in nits.
float pq_eotf(float v) {
    const float m1 = 0.1593017578125;
    const float m2 = 78.84375;
    const float c1 = 0.8359375;
    const float c2 = 18.8515625;
    const float c3 = 18.6875;

    float v_pow = pow(max(v, 1e-6), 1.0 / m2);
    float num = max(v_pow - c1, 0.0);
    float den = max(c2 - c3 * v_pow, 1e-6);
    float lin = pow(num / den, 1.0 / m1);

    return lin; // linear light value, relative (0.0 to 1.0 corresponds to 0 to 10000 nits)
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

vec4 hdr_tonemap(vec4 color) {
    // Apply PQ EOTF to get linear light.
    float r = pq_eotf(color.r);
    float g = pq_eotf(color.g);
    float b = pq_eotf(color.b);

    vec3 linear_rgb = vec3(r, g, b);

    // Convert from BT.2020 to linear sRGB for compositing.
    linear_rgb = bt2020_to_linear_srgb(linear_rgb);

    // Apply tone mapping (simple Reinhard for now).
    float lum = dot(linear_rgb, vec3(0.2126, 0.7152, 0.0722));
    float tone_mapped_lum = lum / (lum + 1.0);
    float scale = tone_mapped_lum / max(lum, 1e-6);
    linear_rgb *= scale;

    // Convert back to sRGB gamma for display.
    vec3 gamma_rgb = pow(clamp(linear_rgb, 0.0, 1.0), vec3(1.0 / 2.2));

    return vec4(gamma_rgb, color.a);
}