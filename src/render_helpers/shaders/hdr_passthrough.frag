//_DEFINES_

// Enable framebuffer fetch for correct alpha blending in PQ space.
// Without this, semi-transparent overlays cause incorrect blending
// because GL blends PQ-encoded values (nonlinear) directly.
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

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

varying vec2 v_coords;

// PQ (ST 2084) constants.
const float PQ_m1 = 0.1593017578125;
const float PQ_m2 = 78.84375;
const float PQ_c1 = 0.8359375;
const float PQ_c2 = 18.8515625;
const float PQ_c3 = 18.6875;

// PQ EOTF: decodes PQ signal back to linear light in nits.
float pq_eotf(float pq) {
    float p = pow(max(pq, 0.0), 1.0 / PQ_m2);
    float num = max(p - PQ_c1, 0.0);
    float den = PQ_c2 - PQ_c3 * p;
    return pow(num / max(den, 1e-6), 1.0 / PQ_m1) * 10000.0;
}

// PQ OETF: converts linear light in nits to PQ-encoded signal.
float pq_oetf(float linear_nits) {
    float l = linear_nits / 10000.0;
    float l_m1 = pow(max(l, 1e-6), PQ_m1);
    float num = PQ_c1 + PQ_c2 * l_m1;
    float den = 1.0 + PQ_c3 * l_m1;
    return pow(num / max(den, 1e-6), PQ_m2);
}

void main() {
    vec4 texel = texture2D(tex, v_coords);

#if defined(NO_ALPHA)
    float src_alpha = alpha;
    vec3 src_pq = texel.rgb;
#else
    float src_alpha = texel.a * alpha;
    vec3 src_pq = (texel.a > 0.001) ? texel.rgb / texel.a : vec3(0.0);
#endif

#ifdef HAS_FRAMEBUFFER_FETCH
    // Decode the existing framebuffer and blend in linear light.
    vec3 fb_pq = gl_LastFragColor.rgb;
    float fb_alpha = gl_LastFragColor.a;

    // Decode framebuffer and source to linear nits.
    vec3 fb_linear = vec3(pq_eotf(fb_pq.r), pq_eotf(fb_pq.g), pq_eotf(fb_pq.b));
    vec3 src_linear = vec3(pq_eotf(src_pq.r), pq_eotf(src_pq.g), pq_eotf(src_pq.b));

    // Alpha blend in linear light.
    vec3 blended = fb_linear * (1.0 - src_alpha) + src_linear * src_alpha;

    // Encode back to PQ.
    vec3 out_pq = vec3(pq_oetf(blended.r), pq_oetf(blended.g), pq_oetf(blended.b));
    gl_FragColor = vec4(out_pq, max(fb_alpha, src_alpha));
#else
    // Fallback: simple premultiplied PQ output (blending in PQ space is incorrect
    // for semi-transparent elements but works for opaque content).
    gl_FragColor = vec4(src_pq * src_alpha, src_alpha);
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        gl_FragColor = vec4(0.0, 0.0, 0.2, 0.2) + gl_FragColor * 0.8;
#endif
}
