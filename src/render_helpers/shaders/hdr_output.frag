#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D niri_tex;
uniform float u_sdr_brightness_nits;
uniform float u_max_nits;

varying vec2 v_tex_coord;

// sRGB to linear conversion.
float srgb_to_linear(float c) {
    if (c <= 0.04045) {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

// Linear to sRGB conversion.
float linear_to_srgb(float c) {
    if (c <= 0.0031308) {
        return c * 12.92;
    } else {
        return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
    }
}

// PQ (ST 2084) OETF: converts linear light in nits to PQ-encoded signal.
float pq_oetf(float linear_nits) {
    const float m1 = 0.1593017578125;
    const float m2 = 78.84375;
    const float c1 = 0.8359375;
    const float c2 = 18.8515625;
    const float c3 = 18.6875;

    // Normalize to 10000 nits reference.
    float l = linear_nits / 10000.0;
    float l_m1 = pow(max(l, 1e-6), m1);
    float num = c1 + c2 * l_m1;
    float den = 1.0 + c3 * l_m1;
    return pow(num / max(den, 1e-6), m2);
}

// Convert linear sRGB to linear BT.2020.
vec3 srgb_to_bt2020(vec3 rgb) {
    mat3 m = mat3(
        0.6274, 0.3293, 0.0433,
        0.0691, 0.9196, 0.0113,
        0.0164, 0.0880, 0.8956
    );
    return m * rgb;
}

void main() {
    // Sample input texture (sRGB gamma).
    vec3 srgb = texture2D(niri_tex, v_tex_coord).rgb;

    // Convert sRGB to linear.
    vec3 linear = vec3(
        srgb_to_linear(srgb.r),
        srgb_to_linear(srgb.g),
        srgb_to_linear(srgb.b)
    );

    // Scale SDR content to target brightness in nits.
    float sdr_scale = u_sdr_brightness_nits / max(u_max_nits, 1.0);
    linear *= sdr_scale;

    // Convert to BT.2020 primaries.
    linear = srgb_to_bt2020(linear);

    // Apply PQ encoding for HDR display.
    vec3 pq = vec3(
        pq_oetf(linear.r),
        pq_oetf(linear.g),
        pq_oetf(linear.b)
    );

    gl_FragColor = vec4(pq, 1.0);
}
