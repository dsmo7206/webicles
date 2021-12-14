#version 300 es

precision mediump float;

in vec4 o_Color;
out vec4 o_FragColor;

void main() {
  o_FragColor = o_Color;
}