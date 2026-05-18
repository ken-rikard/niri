#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D niri_tex;
uniform float u_sdr_brightness_nits;
uniform float u_max_nits;

varying vec2 v_tex_coord;

// Maps SDR content (sRGB gamma) to linear light for blending with HDR content.
// Per BT.2408-7, SDR content is mapped to a specific luminance level (default 203 cd/m²).
vec4 sdr_to_hdr(vec4 color) {
    // Convert sRGB gamma to linear.
    vec3 linear = pow(color.rgb, vec3(2.2));

    // Scale SDR content to the target SDR brightness in nits (relative to max nits).
    float sdr_scale = u_sdr_brightness_nits / max(u_max_nits, 1.0);
    linear *= sdr_scale;

    // Encode back to sRGB gamma for the compositing pipeline.
    vec3 gamma_rgb = pow(clamp(linear, 0.0, 1.0), vec3(1.0 / 2.2));

    return vec4(gamma_rgb, color.a);
}