import { RustCanvas } from "rust-graphics";

const CANVAS_WIDTH = 512;
const CANVAS_HEIGHT = 512;

let isSpawningParticles = false;
let spawnX = 0;
let spawnY = 0;

let canvas = document.createElement("canvas");
canvas.width = CANVAS_WIDTH;
canvas.height = CANVAS_HEIGHT;
document.body.appendChild(canvas);
const ctx = canvas.getContext("2d");

canvas.addEventListener("mousedown", (e) => {
  spawnX = e.offsetX;
  spawnY = e.offsetY;
  isSpawningParticles = true;
});

canvas.addEventListener("mousemove", (e) => {
  spawnX = e.offsetX;
  spawnY = e.offsetY;
});

window.addEventListener("mouseup", (e) => {
  isSpawningParticles = false;
});

const rustCanvas = RustCanvas.new(CANVAS_WIDTH, CANVAS_HEIGHT);
rustCanvas.initialize_particles(20000);

let lastFrameTime = performance.now();
let currentFrameTime = performance.now();
let testBuffer = new ArrayBuffer(CANVAS_WIDTH * CANVAS_HEIGHT * 4);
let view = new Uint32Array(testBuffer);
for (let i = 0; i < view.length; i++) {
  view[i] = 0xff0000ff;
}
let view2 = new Uint8Array(testBuffer);
// let pixelData = Uint8ClampedArray.from(view2);
let pixelData = new Uint8ClampedArray(testBuffer);
let image = new ImageData(pixelData, CANVAS_WIDTH, CANVAS_HEIGHT);
const renderLoop = () => {
  // currentFrameTime = performance.now();
  // if (isSpawningParticles) {
  //   rustCanvas.spawn_particle(spawnX, spawnY, 0, 0);
  // }
  // rustCanvas.update((currentFrameTime - lastFrameTime) / 1000.0);
  // rustCanvas.render(ctx);
  // lastFrameTime = currentFrameTime;
  ctx.putImageData(image, 0, 0);
  requestAnimationFrame(renderLoop);
};
requestAnimationFrame(renderLoop);
