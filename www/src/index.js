"use strict";

import { WasmApp } from "rust-webgl-particles-backend";
import { FramerateDisplay } from "./framerate_display.js";

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


let spawnParticle = () => {
  const randPosRange = 8;
  const randVelRange = 150;
  const spawnX = mouseX + (Math.random() * randPosRange - randPosRange / 2);
  const spawnY = mouseY + (Math.random() * randPosRange - randPosRange / 2);
  const spawnVelX = Math.random() * randVelRange - randVelRange / 2;
  const spawnVelY = Math.random() * randVelRange - randVelRange / 2;
  wasmApp.spawn_particle(spawnX, spawnY, spawnVelX, spawnVelY);
};

// Keeps track of recent fps measurements and updates the fps label
const framerateDisplay = new FramerateDisplay();

// Variables for timing main frame loop
let lastFrameTime = performance.now();
let deltaTime = 0.0;
let timeSinceUpdate = 0.0;

// Callback to "pause" time when window is out of focus
// Essentially, any time passed while the window is out of focus
// is ignored for time calculations in the frame loop
let timestampAtPause = 0.0;
let pauseTimeElapsed = 0.0;
document.addEventListener('visibilitychange', (_event) => {
	if (document.visibilityState === 'hidden') {
		// Store current time on pause
		timestampAtPause = performance.now();
	} else {
		// Push lastFrameTime ahead by however much time elapsed during pause
		// lastFrameTime += performance.now() - timestampAtPause;
		pauseTimeElapsed = performance.now() - timestampAtPause;
		console.log('pauseTimeElapsed on focus: ', pauseTimeElapsed);
	}
});

// Main frame redraw and update loop
const animationFrameLoop = (currentFrameTime) => {
	let debugLastFrameTime = lastFrameTime;
	let debugPauseTimeElapsed = pauseTimeElapsed;
	lastFrameTime += pauseTimeElapsed;
	pauseTimeElapsed = 0;
	deltaTime = currentFrameTime - lastFrameTime;
	if (deltaTime > 1000) {
		console.log('lastFrameTime: ', debugLastFrameTime);
		console.log('pauseTimeElapsed: ', debugPauseTimeElapsed);
		console.log('deltaTime: ', deltaTime);
	}
	lastFrameTime = currentFrameTime;

	// Update labels in case some values have changed
	framerateDisplay.update(deltaTime);
	updateParticleCountLabel();
	updateSimSpeedLabel();

	// This ensures that the physics timestep always runs at 60 fps (16.7 ms)
	// If the simulation speed (simTicksPerFrame) is higher than 1x
	// it will adjust the amount of physics steps accordingly
	timeSinceUpdate += deltaTime;
	let updatesThisFrame = 0;
	while (timeSinceUpdate >= (16.7/simTicksPerFrame) &&
		updatesThisFrame < MAX_UPDATES_PER_FRAME) {
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

	// Begins the frame loop again
	requestAnimationFrame(animationFrameLoop);
};

// Frame loop actually starts here
requestAnimationFrame(animationFrameLoop);


// Functions related to the UI, including callbacks for all html input elements
function updateParticleCountLabel() {
	document.getElementById("particle-count-label").textContent =
		`Particles: ${wasmApp.get_particle_count()}`;
}

function updateSimSpeedLabel() {
	document.getElementById(
		"sim-speed-label"
	).textContent = `Simulation Speed: x${simTicksPerFrame}`;
}

function connectUICallbacks() {
	// Gravity Well Mass Slider
	document.getElementById("gravity-well-mass-slider").onchange = function() {
		wasmApp.set_gravity_well_mass(this.value);
	}

	// Clear Particles Button
	document.getElementById("clear-particles-button").onclick = function() {
		wasmApp.clear_particles();
	}

	// Borders Active Checkbox
	document.getElementById("borders-active-checkbox").onclick = function() {
		wasmApp.set_borders_active(this.checked);
	}

	// Clear Screen Checkbox
	document.getElementById("clear-screen-checkbox").onclick = function() {
		wasmApp.set_should_clear_screen(this.checked);
	}

	// Sim Speed Down Button
	document.getElementById("sim-speed-down-button").onclick = function() {
		if (simTicksPerFrame > 1) {
			simTicksPerFrame -= 1;
		}
		updateSimSpeedLabel();
	}

	// Sim Speed Up Button
	document.getElementById("sim-speed-up-button").onclick = function() {
		simTicksPerFrame += 1;
		updateSimSpeedLabel();
	}

	// Trail Scale Slider
	document.getElementById("trail-scale-slider").onchange = function() {
		wasmApp.set_particle_trail_scale(this.value);
	}

	// Remove Some Particles Button
	document.getElementById("remove-some-particles-button").onclick = function() {
		wasmApp.remove_particles(250);
	}
}

// Set up mouse interaction through canvas events
function addEventCallbacksToCanvas(canvas) {
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

	canvas.addEventListener("pointerup", (e) => {
		if (e.button === 0) {
			isSpawningParticles = false;
			if (isDragging) {
				isDragging = false;
				wasmApp.release_selection();
			}
		}
	});
}

// Helper function that randomizes spawned particle's starting
// offset and velocity
function spawnParticle() {
	const randPosRange = 8;
	const randVelRange = 150;
	const spawnX = mouseX + (Math.random() * randPosRange - randPosRange / 2);
	const spawnY = mouseY + (Math.random() * randPosRange - randPosRange / 2);
	const spawnVelX = Math.random() * randVelRange - randVelRange / 2;
	const spawnVelY = Math.random() * randVelRange - randVelRange / 2;
	wasmApp.spawn_particle(spawnX, spawnY, spawnVelX, spawnVelY);
}
