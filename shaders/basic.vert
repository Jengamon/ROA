#version 330 core

in vec2 position;
in vec4 color;

out vec4 v_color;

uniform mat4 mvp;

void main() {
  // v_tex_coords = tex_coords;
  v_color = color;
  gl_Position = mvp * vec4(position, 0, 1);
}
