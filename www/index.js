import { RustCanvas } from "rust-graphics";
import { memory } from "rust-graphics/rust_graphics_bg";

const CANVAS_WIDTH = 512;
const CANVAS_HEIGHT = 512;

const MAX_UPDATES_PER_FRAME = 50;

let isSpawningParticles = false;
let particleSpawnRate = 5;
let particleTrailLength = 5;
let isDragging = false;
let mouseX = 0;
let mouseY = 0;
let simTicksPerFrame = 1;

// let canvas = document.createElement("canvas");
// canvas.width = CANVAS_WIDTH;
// canvas.height = CANVAS_HEIGHT;
// document.body.appendChild(canvas);
let canvas = document.getElementById("game-canvas");
const ctx = canvas.getContext("2d");

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

let particleTrailLengthDownButton = document.getElementById(
  "particle-trail-length-down"
);
let particleTrailLengthUpButton = document.getElementById(
  "particle-trail-length-up"
);
particleTrailLengthDownButton.addEventListener("click", (e) => {
  if (particleTrailLength > 1) {
    particleTrailLength -= 1;
  }
  rustCanvas.set_particle_trail_length(particleTrailLength);
  updateParticleTrailLengthLabel();
});
particleTrailLengthUpButton.addEventListener("click", (e) => {
  particleTrailLength += 1;
  rustCanvas.set_particle_trail_length(particleTrailLength);
  updateParticleTrailLengthLabel();
});
const updateParticleTrailLengthLabel = () => {
  document.getElementById(
    "particle-trail-length-label"
  ).textContent = `Particle Trail Length: ${particleTrailLength}`;
};

window.oncontextmenu = (e) => {
  e.preventDefault();
};

canvas.addEventListener("pointerdown", (e) => {
  if (e.button === 0) {
    if (e.ctrlKey) {
      rustCanvas.spawn_gravity_well(mouseX, mouseY);
    } else {
      if (rustCanvas.try_selecting(mouseX, mouseY)) {
        isDragging = true;
      } else {
        isSpawningParticles = true;
      }
    }
  } else if (e.button === 2) {
    rustCanvas.try_removing(mouseX, mouseY);
  }
});

canvas.addEventListener("pointermove", (e) => {
  mouseX = e.offsetX;
  mouseY = e.offsetY;
  if (isDragging) {
    rustCanvas.drag_selection(e.movementX, e.movementY);
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

// const rustCanvas = RustCanvas.new(CANVAS_WIDTH, CANVAS_HEIGHT);
const rustCanvas = RustCanvas.new(canvas.width, canvas.height);
rustCanvas.spawn_gravity_well(canvas.width / 2.0, canvas.height / 2.0);
// rustCanvas.initialize_particles(10);

let pixelDataPtr = rustCanvas.get_pixel_buffer_ptr();
let pixelData = new Uint8ClampedArray(
  memory.buffer,
  pixelDataPtr,
  // CANVAS_WIDTH * CANVAS_HEIGHT * 4
  canvas.width * canvas.height * 4
);

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
  let randPosRange = 8;
  let randVelRange = 60;
  let spawnX = mouseX + (Math.random() * randPosRange - randPosRange / 2);
  let spawnY = mouseY + (Math.random() * randPosRange - randPosRange / 2);
  let spawnVelX = Math.random() * randVelRange - randVelRange / 2;
  let spawnVelY = Math.random() * randVelRange - randVelRange / 2;
  rustCanvas.spawn_particle(spawnX, spawnY, spawnVelX, spawnVelY);
};

let updateParticleCountLabel = () => {
  particleCountElement.textContent = `Particles: ${rustCanvas.get_particle_count()}`;
};

let currentFrameTime = performance.now();
let timeSinceUpdate = 0.0;
let timeSinceRender = 0.0;
const renderLoop = () => {
  frameRateCounter.render();
  let lastFrameTime = currentFrameTime;
  currentFrameTime = performance.now();
  let frameDelta = currentFrameTime - lastFrameTime;
  timeSinceUpdate += frameDelta;
  timeSinceRender += frameDelta;
  updateParticleCountLabel();
  updateSimSpeedLabel();
  updateParticleTrailLengthLabel();
  let updatesThisFrame = 0;
  do {
    for (let i = 0; i < simTicksPerFrame; i++) {
      if (isSpawningParticles) {
        for (let i = 0; i < particleSpawnRate; i++) {
          spawnParticle();
        }
      }
      rustCanvas.update(16.7);
    }
    timeSinceUpdate -= 16.7;
    updatesThisFrame++;
  } while (
    timeSinceUpdate >= 16.7 &&
    updatesThisFrame <= MAX_UPDATES_PER_FRAME
  );
  // if (timeSinceRender >= 33.4) {
  rustCanvas.render();
  ctx.putImageData(image, 0, 0);
  timeSinceRender = 0.0;
  // }
  requestAnimationFrame(renderLoop);
};
requestAnimationFrame(renderLoop);
