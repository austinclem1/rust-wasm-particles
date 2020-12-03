"use strict";

import { RustCanvas } from "rust-graphics";
import { memory } from "rust-graphics/rust_graphics_bg";

// const CANVAS_WIDTH = 512;
// const CANVAS_HEIGHT = 512;

const MAX_UPDATES_PER_FRAME = 10;

let isSpawningParticles = false;
let particleSpawnRate = 5;
let isDragging = false;
let mouseX = 0;
let mouseY = 0;
let simTicksPerFrame = 1;

// let canvas = document.createElement("canvas");
// canvas.width = CANVAS_WIDTH;
// canvas.height = CANVAS_HEIGHT;
// document.body.appendChild(canvas);
// let canvas = document.getElementById("game-canvas");
// const ctx = canvas.getContext("2d");

let gravityWellMassSlider = document.getElementById("gravity-well-mass");
gravityWellMassSlider.addEventListener("change", (e) => {
  rustCanvas.set_gravity_well_mass(gravityWellMassSlider.value);
});

let particleCountElement = document.getElementById("particle-count");

let clearParticlesButton = document.getElementById("clear-particles-button");
clearParticlesButton.addEventListener("click", (e) => {
  rustCanvas.clear_particles();
});

let removeSomeParticlesButton = document.getElementById(
  "remove-some-particles-button"
);
removeSomeParticlesButton.addEventListener("click", (e) => {
  rustCanvas.remove_particles(250);
});

let bordersActiveCheckbox = document.getElementById("borders-active-checkbox");
bordersActiveCheckbox.addEventListener("input", (e) => {
  rustCanvas.set_borders_active(bordersActiveCheckbox.checked);
});

let screenClearCheckbox = document.getElementById("clear-screen-checkbox");
screenClearCheckbox.addEventListener("input", (e) => {
  rustCanvas.set_should_clear_screen(screenClearCheckbox.checked);
});

let simSpeedDownButton = document.getElementById("sim-speed-down");
let simSpeedUpButton = document.getElementById("sim-speed-up");
simSpeedDownButton.addEventListener("click", (e) => {
  if (simTicksPerFrame > 1) {
    simTicksPerFrame -= 1;
  }
  updateSimSpeedLabel();
});
simSpeedUpButton.addEventListener("click", (e) => {
  simTicksPerFrame += 1;
  updateSimSpeedLabel();
});
const updateSimSpeedLabel = () => {
  document.getElementById(
    "sim-speed-label"
  ).textContent = `Simulation Speed: x${simTicksPerFrame}`;
};

// const updateParticleTrailLengthLabel = () => {
//   document.getElementById(
//     "particle-trail-length-label"
//   ).textContent = `Particle Trail Length: ${particleTrailLength}`;
// };
let trailScaleSlider = document.getElementById("particle-trail-scale");
trailScaleSlider.addEventListener("change", (e) => {
  rustCanvas.set_particle_trail_scale(trailScaleSlider.value);
});

window.oncontextmenu = (e) => {
  e.preventDefault();
};

canvas.addEventListener("pointerdown", (e) => {
  if (e.button === 0) {
    if (e.ctrlKey) {
      rustCanvas.spawn_gravity_well(e.offsetX, e.offsetY);
    } else {
      if (rustCanvas.try_selecting(e.offsetX, e.offsetY)) {
        isDragging = true;
      } else {
        isSpawningParticles = true;
      }
    }
  } else if (e.button === 2) {
    rustCanvas.try_removing(e.offsetX, e.offsetY);
  }
});

canvas.addEventListener("pointermove", (e) => {
  mouseX = e.offsetX;
  mouseY = e.offsetY;
  if (isDragging) {
    rustCanvas.move_selection_to(e.offsetX, e.offsetY);
  }
});

window.addEventListener("pointerup", (e) => {
  if (e.button === 0) {
    isSpawningParticles = false;
    if (isDragging) {
      isDragging = false;
      rustCanvas.release_selection();
    }
  }
});

const rustCanvas = RustCanvas.new();
rustCanvas.initialize();
let image = new Image();
image.onload = function() {
    rustCanvas.add_texture_from_image("gravity_well", image);
};
image.src = './spiral.png';
// image.src = './gravity_well.bmp';
// image.src = 'https://raw.githubusercontent.com/austinclem1/austinclem1.github.io/main/assets/spiral.png';
// image.src = 'https://homepages.cae.wisc.edu/~ece533/images/boy.bmp';
rustCanvas.spawn_gravity_well(canvas.width / 2.0, canvas.height / 2.0);
rustCanvas.initialize_particles(10000);

const frameRateCounter = new (class {
  constructor() {
    this.fpsElement = document.getElementById("fps");
    this.frames = [];
    this.lastFrameTimeStamp = performance.now();
  }

  render() {
    const now = performance.now();
    const delta = now - this.lastFrameTimeStamp;
    this.lastFrameTimeStamp = now;
    const fps = (1 / delta) * 1000;

    this.frames.push(fps);
    if (this.frames.length > 100) {
      this.frames.shift();
    }

    let min = Infinity;
    let max = -Infinity;
    let sum = 0;
    for (let i = 0; i < this.frames.length; i++) {
      sum += this.frames[i];
      min = Math.min(this.frames[i], min);
      max = Math.max(this.frames[i], max);
    }
    let mean = sum / this.frames.length;

    this.fpsElement.textContent = `
Frames per Second:
avg of last 100: ${Math.round(mean)}\n
min of last 100: ${Math.round(min)}\n
`.trim();
  }
})();

let spawnParticle = () => {
  const randPosRange = 8;
  const randVelRange = 150;
  const spawnX = mouseX + (Math.random() * randPosRange - randPosRange / 2);
  const spawnY = mouseY + (Math.random() * randPosRange - randPosRange / 2);
  const spawnVelX = Math.random() * randVelRange - randVelRange / 2;
  const spawnVelY = Math.random() * randVelRange - randVelRange / 2;
  rustCanvas.spawn_particle(spawnX, spawnY, spawnVelX, spawnVelY);
};

let updateParticleCountLabel = () => {
  particleCountElement.textContent = `Particles: ${rustCanvas.get_particle_count()}`;
};

let lastFrameTime = performance.now();
let timeSinceUpdate = 0.0;
let deltaTime = 0.0;
let updatesThisFrame;
const animationFrameLoop = (currentFrameTime) => {
  frameRateCounter.render();
  deltaTime = currentFrameTime - lastFrameTime;
  lastFrameTime = currentFrameTime;
  updateParticleCountLabel();
  updateSimSpeedLabel();
  timeSinceUpdate += deltaTime;
  updatesThisFrame = 0;
  while (timeSinceUpdate >= (16.7/simTicksPerFrame) &&
    updatesThisFrame <= MAX_UPDATES_PER_FRAME) {
    for (let i = 0; i < simTicksPerFrame; i++) {
      rustCanvas.update_1(16.7);
      if (isSpawningParticles) {
        for (let j = 0; j < particleSpawnRate; j++) {
          spawnParticle();
        }
      }
      updatesThisFrame++;
    }
    timeSinceUpdate -= 16.7;
  }
  // rustCanvas.update_1(deltaTime * simTicksPerFrame);
  // if (isSpawningParticles) {
  //   for (let i = 0; i < particleSpawnRate * simTicksPerFrame; i++) {
  //     spawnParticle();
  //   }
  // }
  rustCanvas.render();
  requestAnimationFrame(animationFrameLoop);
};

// BENCHING
// let update_1_start = performance.now();
// rustCanvas.clear_particles();
// for (let i = 0; i < 1; i++) {
//   rustCanvas.initialize_particles(100000);
//   for (let j = 0; j < 600; j++) {
//     rustCanvas.update_1(16.7);
//   }
// }
// let update_1_elapsed = performance.now() - update_1_start;

// let update_2_start = performance.now();
// rustCanvas.clear_particles();
// for (let i = 0; i < 1; i++) {
//   rustCanvas.initialize_particles(100000);
//   for (let j = 0; j < 600; j++) {
//     rustCanvas.update_2(16.7);
//   }
// }
// let update_2_elapsed = performance.now() - update_2_start;

// console.log("Update 1 time elapsed: ", update_1_elapsed);
// console.log("Update 2 time elapsed: ", update_2_elapsed);


requestAnimationFrame(animationFrameLoop);
