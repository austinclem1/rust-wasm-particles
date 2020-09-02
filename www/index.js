import { RustCanvas } from "rust-graphics";
import { memory } from "rust-graphics/rust_graphics_bg";

const CANVAS_WIDTH = 512;
const CANVAS_HEIGHT = 512;

const MAX_UPDATES_PER_FRAME = 50;

let isSpawningParticles = false;
let isDragging = false;
let mouseX = 0;
let mouseY = 0;

let canvas = document.createElement("canvas");
canvas.width = CANVAS_WIDTH;
canvas.height = CANVAS_HEIGHT;
document.body.appendChild(canvas);
const ctx = canvas.getContext("2d");

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
      rustCanvas.release_selection();
      isDragging = false;
    }
  }
});

const rustCanvas = RustCanvas.new(CANVAS_WIDTH, CANVAS_HEIGHT);
rustCanvas.initialize_particles(10);

let pixelDataPtr = rustCanvas.get_pixel_buffer_ptr();
let pixelData = new Uint8ClampedArray(
  memory.buffer,
  pixelDataPtr,
  CANVAS_WIDTH * CANVAS_HEIGHT * 4
);
let image = new ImageData(pixelData, CANVAS_WIDTH, CANVAS_HEIGHT);
let currentFrameTime = performance.now();
let timeSinceUpdate = 0.0;
let timeSinceRender = 0.0;
const renderLoop = () => {
  let lastFrameTime = currentFrameTime;
  currentFrameTime = performance.now();
  let frameDelta = currentFrameTime - lastFrameTime;
  timeSinceUpdate += frameDelta;
  timeSinceRender += frameDelta;
  let updatesThisFrame = 0;
  do {
    if (isSpawningParticles) {
      for (let i = 0; i < 5; i++) {
        let randPosRange = 8;
        let randVelRange = 60;
        let spawnX = mouseX + (Math.random() * randPosRange - randPosRange / 2);
        let spawnY = mouseY + (Math.random() * randPosRange - randPosRange / 2);
        let spawnVelX = Math.random() * randVelRange - randVelRange / 2;
        let spawnVelY = Math.random() * randVelRange - randVelRange / 2;
        rustCanvas.spawn_particle(spawnX, spawnY, spawnVelX, spawnVelY);
      }
    }
    // rustCanvas.update(frameDelta);
    rustCanvas.update(16.7);
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
