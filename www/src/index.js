"use strict";

import { WasmApp } from "rust-webgl-particles-backend";

const MAX_UPDATES_PER_FRAME = 10;

let isSpawningParticles = false;
let particleSpawnRate = 5;
let isDragging = false;
let mouseX = 0;
let mouseY = 0;
let simTicksPerFrame = 1;

// const updateParticleTrailLengthLabel = () => {
//   document.getElementById(
//     "particle-trail-length-label"
//   ).textContent = `Particle Trail Length: ${particleTrailLength}`;
// };

window.oncontextmenu = (e) => {
  e.preventDefault();
};

const canvas = document.getElementById("canvas");

canvas.addEventListener("pointerdown", (e) => {
  if (e.button === 0) {
    if (e.ctrlKey) {
      wasmApp.spawn_gravity_well(e.offsetX, e.offsetY);
    } else {
      if (wasmApp.try_selecting(e.offsetX, e.offsetY)) {
        isDragging = true;
      } else {
        isSpawningParticles = true;
      }
    }
  } else if (e.button === 2) {
    wasmApp.try_removing(e.offsetX, e.offsetY);
  }
});

canvas.addEventListener("pointermove", (e) => {
  mouseX = e.offsetX;
  mouseY = e.offsetY;
  if (isDragging) {
    wasmApp.move_selection_to(e.offsetX, e.offsetY);
  }
});

window.addEventListener("pointerup", (e) => {
  if (e.button === 0) {
    isSpawningParticles = false;
    if (isDragging) {
      isDragging = false;
      wasmApp.release_selection();
    }
  }
});

const wasmApp = WasmApp.new();
// wasmApp.initialize_canvas();
wasmApp.connect_canvas_element(canvas);
let image = new Image();
image.src = '../assets/spiral.png';
image.addEventListener('load', function() {
	wasmApp.add_texture_from_image("gravity_well", image);
});
// image.onload = function() {
//     wasmApp.add_texture_from_image("gravity_well", image);
// };
// image.src = './gravity_well.bmp';
// image.src = 'https://raw.githubusercontent.com/austinclem1/austinclem1.github.io/main/assets/spiral.png';
// image.src = 'https://homepages.cae.wisc.edu/~ece533/images/boy.bmp';
wasmApp.spawn_gravity_well(canvas.width / 2.0, canvas.height / 2.0);
wasmApp.initialize_particles(10000);

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
  wasmApp.spawn_particle(spawnX, spawnY, spawnVelX, spawnVelY);
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
      wasmApp.update(16.7);
      if (isSpawningParticles) {
        for (let j = 0; j < particleSpawnRate; j++) {
          spawnParticle();
        }
      }
      updatesThisFrame++;
    }
    timeSinceUpdate -= 16.7;
  }
  wasmApp.render();
  requestAnimationFrame(animationFrameLoop);
};

requestAnimationFrame(animationFrameLoop);

function updateParticleCountLabel() {
  document.getElementById("particle-count-label").textContent = `Particles: ${wasmApp.get_particle_count()}`;
}

function onChangeGravityWellMassSlider() {
  wasmApp.set_gravity_well_mass(gravityWellMassSlider.value);
}

function onClickClearParticlesButton() {
  wasmApp.clear_particles();
}

function onClickRemoveSomeParticlesButton() {
  wasmApp.remove_particles(250);
}

function onClickBordersActiveCheckbox(checkbox) {
  wasmApp.set_borders_active(checkbox.checked);
}

function onClickScreenClearCheckbox(checkbox) {
  wasmApp.set_should_clear_screen(checkbox.checked);
}

function onClickSimSpeedDownButton() {
  if (simTicksPerFrame > 1) {
    simTicksPerFrame -= 1;
  }
  updateSimSpeedLabel();
}

function onClickSimSpeedUpButton() {
  simTicksPerFrame += 1;
  updateSimSpeedLabel();
}

function updateSimSpeedLabel() {
  document.getElementById(
    "sim-speed-label"
  ).textContent = `Simulation Speed: x${simTicksPerFrame}`;
}

function onChangeTrailScaleSlider() {
  wasmApp.set_particle_trail_scale(trailScaleSlider.value);
}
