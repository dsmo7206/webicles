#version 300 es

precision mediump float;

uniform float u_TimeDelta; // In seconds
uniform sampler2D u_RgNoise;
uniform vec2 u_Gravity;
uniform vec2 u_Origin; // Where particles start
uniform float u_MinTheta; // Range of dirs from (1, 0) a new particle can head in.
uniform float u_MaxTheta; // For all directions, use (-PI, PI)
uniform float u_MinSpeed;
uniform float u_MaxSpeed;

/* Inputs. These reflect the state of a single particle before the update. */
in vec2 i_Position; // Where the particle is
in float i_Age; // Age of particle in seconds
in float i_Life; // How long this particle is supposed to live
in vec2 i_Velocity; // Which direction it is moving, and how fast

/* Outputs. These mirror the inputs. These values will be captured
   into our transform feedback buffer! */
out vec2 v_Position;
out float v_Age;
out float v_Life;
out vec2 v_Velocity;

void main() {
  if (i_Age >= i_Life) {
    /* Particle has exceeded its lifetime! Time to spawn a new one
       in place of the old one, in accordance with our rules.*/
    
    /* First, choose where to sample the random texture. I do it here
       based on particle ID. It means that basically, you're going to
       get the same initial random values for a given particle. The result
       still looks good. I suppose you could get fancier, and sample
       based on particle ID *and* time, or even have a texture where values
       are not-so-random, to control the pattern of generation. */
    ivec2 noise_coord = ivec2(gl_VertexID % 512, gl_VertexID / 512);
    
    /* Get the pair of random values. */
    vec2 rand = texelFetch(u_RgNoise, noise_coord, 0).rg;

    /* Decide the direction of the particle based on the first random value.
       The direction is determined by the angle theta that its vector makes
       with the vector (1, 0).*/
    float theta = u_MinTheta + rand.r*(u_MaxTheta - u_MinTheta);

    /* Derive the x and y components of the direction unit vector.
       This is just basic trig. */
    float x = cos(theta);
    float y = sin(theta);

    /* Return the particle to origin. */
    v_Position = u_Origin;

    /* It's new, so age must be set accordingly.*/
    v_Age = 0.0;
    v_Life = i_Life;

    /* Generate final velocity vector. We use the second random value here
       to randomize speed. */
    v_Velocity =
      vec2(x, y) * (u_MinSpeed + rand.g * (u_MaxSpeed - u_MinSpeed));

  } else {
    /* Update parameters according to our simple rules.*/
    v_Position = i_Position + i_Velocity * u_TimeDelta;
    v_Age = i_Age + u_TimeDelta;
    v_Life = i_Life;
    v_Velocity = i_Velocity + u_Gravity * u_TimeDelta;
  }
}