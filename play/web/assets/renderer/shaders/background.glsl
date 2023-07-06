varying vec2 v_uv;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform vec2 u_scale;
uniform vec2 u_parallax;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;
vec2 world_pos(vec2 v) {
    vec3 w = (inverse(u_projection_matrix * u_view_matrix) * vec3(v, 1.0));
    return w.xy / w.z;
}
void main() {
    v_uv = (world_pos(a_pos) - world_pos(vec2(0.0, 0.0)) * u_parallax) / u_scale;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform ivec2 u_texture_size;
void main() {
    gl_FragColor = smoothTexture2D(v_uv, u_texture, u_texture_size);
}
#endif
