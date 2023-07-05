varying vec2 v_pos;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
void main() {
    v_pos = a_pos;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform vec4 u_color;
uniform float u_inner_radius;
uniform float u_outer_radius;
void main() {
    gl_FragColor = premultiply_alpha(u_color) * clamp((length(v_pos) - u_inner_radius) / (u_outer_radius - u_inner_radius), 0.0, 1.0);
}
#endif
