//_DEFINES_

// Passthrough shader for native HDR content.
// Assumes input is already PQ-encoded BT.2020 data.
// Just passes through with alpha blending.

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

void main() {
    vec4 texel = texture2D(tex, v_coords);

#if defined(NO_ALPHA)
    float src_alpha = alpha;
    vec3 src = texel.rgb;
#else
    float src_alpha = texel.a * alpha;
    vec3 src = (texel.a > 0.001) ? texel.rgb / texel.a : vec3(0.0);
#endif

    // Content is already PQ-encoded, just apply alpha.
    gl_FragColor = vec4(src * src_alpha, src_alpha);

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        gl_FragColor = vec4(0.0, 0.0, 0.2, 0.2) + gl_FragColor * 0.8;
#endif
}