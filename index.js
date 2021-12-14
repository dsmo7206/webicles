import * as wasm from "./pkg";
wasm.start();

const fps = document.getElementById("fps");

fps.innerHTML = "Loading...";
let numFrames = 0;
let lastTime = Date.now();

const tri = wasm.Triangle.new(
  document.getElementById("canvas"),
  wasm.Colour.new(1, 0, 0, 1),
  wasm.Colour.new(0, 1, 0, 1)
);

// const sp = wasm.StaticParticles.new(
//   document.getElementById("canvas2"),
//   1024, // num_particles
//   100000, // particle_birth_rate
//   0.0, // gravity_x
//   -1.0 // gravity_y
// );

const mpm = wasm.RustMlsMpm.new(document.getElementById("canvas2"), 100, 40);

const renderLoop = () => {
  tri.draw();
  //sp.draw(0.01);
  mpm.draw(1e-4);
  animationId = requestAnimationFrame(renderLoop);
  numFrames += 1;

  if (numFrames % 100 === 0) {
    const currTime = Date.now();
    const elapsed = currTime - lastTime;
    // 100 frames happened in `elapsed` ms
    lastTime = currTime;
    const thisFps = 100000 / elapsed;
    fps.innerHTML = "FPS: " + thisFps;
  }
};

let animationId = requestAnimationFrame(renderLoop);
