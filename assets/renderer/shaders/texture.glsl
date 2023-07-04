varying vec2 v_uv;
varying vec2 v_tile_uv_bottom_left;
varying vec2 v_tile_uv_top_right;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_uv;
attribute vec2 a_tile_uv_bottom_left;
attribute vec2 a_tile_uv_top_right;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_model_matrix;
void main() {
    v_tile_uv_bottom_left = a_tile_uv_bottom_left;
    v_tile_uv_top_right = a_tile_uv_top_right;
    v_uv = a_uv;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform vec4 u_color;
uniform sampler2D u_texture;
uniform vec2 u_texture_size;
void main() {
    gl_FragColor = u_color * texture2D(u_texture, clamp(v_uv, v_tile_uv_bottom_left + 0.5 / u_texture_size, v_tile_uv_top_right - 0.5 / u_texture_size));
}
#endif
