varying vec2 v_uv;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_uv;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_model_matrix;
void main() {
    v_uv = a_uv;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER

uniform vec4 u_color;
uniform sampler2D u_texture;
uniform ivec2 u_texture_size;
void main() {
    gl_FragColor = u_color * smoothTexture2D(v_uv, u_texture, u_texture_size);
}
#endif
