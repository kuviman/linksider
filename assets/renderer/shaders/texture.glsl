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

// https://www.youtube.com/watch?v=d6tp43wZqps
vec4 smoothTexture2D(vec2 data_uv, sampler2D tex, vec2 tex_size) {
    // box filter size in texel units
    vec2 box_size = clamp(fwidth(data_uv) * tex_size, 1e-5, 1.0);
    // scale uv by texture size to get texel coordinate
    vec2 tx = data_uv * tex_size - 0.5 * box_size;
    // compute offset for pixel-sized box filter
    vec2 tx_offset = max((fract(tx) - (1.0 - box_size)) / box_size, 0.0);
    // compute bilinear sample uv coordinates
    vec2 uv = (floor(tx) + 0.5 + tx_offset) / tex_size;
    // sample the texture
    return texture2D(tex, uv);
}

uniform vec4 u_color;
uniform sampler2D u_texture;
uniform vec2 u_texture_size;
void main() {
    vec2 clamped_uv = clamp(v_uv, v_tile_uv_bottom_left + 0.5 / u_texture_size, v_tile_uv_top_right - 0.5 / u_texture_size);
    // gl_FragColor = u_color * texture2D(u_texture, clamped_uv);
    gl_FragColor = u_color * smoothTexture2D(clamped_uv, u_texture, u_texture_size);
}
#endif
