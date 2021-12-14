#version 300 es

precision mediump float;

in vec2 i_Position;

void main() {
  gl_PointSize = 5.0;
  gl_Position = vec4((i_Position  * vec2(2,2)- vec2(1, 1)), 0.0, 1.0);
}