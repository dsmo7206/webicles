#version 300 es
    
precision highp float;

uniform vec4 fg_colour;
out vec4 out_colour;

void main() {
    out_colour = vec4(fg_colour);
}
