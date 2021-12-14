#version 300 es

precision mediump float;

in vec2 i_Position;
in float i_Age;
in float i_Life;
in vec2 i_Velocity;

out float v_Age;
out float v_Life;

void main() {
  /* Set varyings so that frag shader can use these values too.*/
  v_Age = i_Age;
  v_Life = i_Life;

  /* Vary point size based on age. Make old particles shrink. */
  gl_PointSize = 1.0 + 6.0 * (1.0 - i_Age/i_Life);

  gl_Position = vec4(i_Position, 0.0, 1.0);
}