#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D tex;
uniform float u_sdr_brightness_nits;
uniform float u_max_nits;
uniform float u_sdr_color_intensity;

varying vec2 v_coords;

void main() {
    // Just pass through the texture as-is (HDR pipeline test)
    gl_FragColor = texture2D(tex, v_coords);
}