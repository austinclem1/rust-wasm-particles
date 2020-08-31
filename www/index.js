import { RustCanvas } from "rust-graphics";
import { memory } from "rust-graphics/rust_graphics_bg";

const CANVAS_WIDTH = 512;
const CANVAS_HEIGHT = 512;

let isSpawningParticles = false;
let mouseX = 0;
let mouseY = 0;

let canvas = document.createElement("canvas");
canvas.width = CANVAS_WIDTH;
canvas.height = CANVAS_HEIGHT;
document.body.appendChild(canvas);
const ctx = canvas.getContext("2d");

canvas.addEventListener("pointerdown", (e) => {
  mouseX = e.offsetX;
  mouseY = e.offsetY;
  isSpawningParticles = true;
});

canvas.addEventListener("pointermove", (e) => {
  mouseX = e.offsetX;
  mouseY = e.offsetY;
});

window.addEventListener("pointerup", (e) => {
  isSpawningParticles = false;
});

const rustCanvas = RustCanvas.new(CANVAS_WIDTH, CANVAS_HEIGHT);
rustCanvas.initialize_particles(200);

let pixelDataPtr = rustCanvas.get_pixel_data_ptr();
let pixelData = new Uint8ClampedArray(
  memory.buffer,
  pixelDataPtr,
  CANVAS_WIDTH * CANVAS_HEIGHT * 4
);
let image = new ImageData(pixelData, CANVAS_WIDTH, CANVAS_HEIGHT);
let currentFrameTime = performance.now();
const renderLoop = () => {
  let lastFrameTime = currentFrameTime;
  currentFrameTime = performance.now();
  let frameDelta = (currentFrameTime - lastFrameTime) / 1000.0;
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
  rustCanvas.update(frameDelta);
  rustCanvas.render();
  ctx.putImageData(image, 0, 0);
  requestAnimationFrame(renderLoop);
};
requestAnimationFrame(renderLoop);
