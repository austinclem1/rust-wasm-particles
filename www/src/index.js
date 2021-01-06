"use strict";

import { WasmApp } from "rust-webgl-particles-backend";
import { FramerateDisplay } from "./framerate_display.js";

// Globals for mouse position
let mouseX = 0;
let mouseY = 0;

// How many physics steps to simulate per graphics rendering frame
let simTicksPerFrame = 1;
// Prevents "spiral of death" loop if the cpu can't keep up with
// physics updates, in the worst case the physics will now slow down
// if we've updated MAX_UPDATES_PER_FRAME steps this graphical frame
const MAX_UPDATES_PER_FRAME = 10;

// How fast particles spawn when the user holds left mouse button
const particleSpawnRate = 5;

// More globals for spawning particles and dragging gravity wells
let isSpawningParticles = false;
let isDragging = false;

const canvas = document.getElementById("canvas");

// Prevent right-click menu from showing on canvas
// so we can right-click on the gravity well to delete it
canvas.oncontextmenu = (e) => {
	e.preventDefault();
};

// Improve touch screen interaction
canvas.style.touchAction = "none";

// Set up UI with helper functions
addEventCallbacksToCanvas(canvas);
connectUICallbacks();

// Here is our wasm backend instance that handles
// the actual particle simulation
const wasmApp = WasmApp.new();
wasmApp.connect_canvas_element(canvas);

// Load the gravity well image and store it as a webGl texture
// in the wasm app
{
	const image = new Image();
	image.src = '../assets/spiral.png';
	image.addEventListener('load', function() {
		wasmApp.add_texture_from_image("gravity_well", image);
	});
}

// Initialize canvas with one gravity well in the center, and some particles
wasmApp.spawn_gravity_well(canvas.width / 2.0, canvas.height / 2.0);
wasmApp.initialize_particles(10000);

// Keeps track of recent fps measurements and updates the fps label
const framerateDisplay = new FramerateDisplay();

// Variables for timing main frame loop
let lastFrameTime = performance.now();
let deltaTime = 0.0;
let timeSinceUpdate = 0.0;

// Callback to "pause" time when window is out of focus
// Essentially, any time passed while the window is out of focus
// is ignored for time calculations in the frame loop once we return focus
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
	}
});

// Main frame redraw and update loop
const animationFrameLoop = (currentFrameTime) => {
	lastFrameTime += pauseTimeElapsed;
	pauseTimeElapsed = 0;
	deltaTime = currentFrameTime - lastFrameTime;
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
		mouseX = (e.pageX - canvas.offsetLeft);
		mouseY = (e.pageY - canvas.offsetTop);
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
		// Calculate how much mouse has moved since last frame
		// and use that to move gravity well if it is being dragged
		let movementX = (e.pageX - canvas.offsetLeft) - mouseX;
		let movementY = (e.pageY - canvas.offsetTop) - mouseY;
		mouseX = (e.pageX - canvas.offsetLeft);
		mouseY = (e.pageY - canvas.offsetTop);
		if (isDragging) {
			wasmApp.move_selection_by(movementX, movementY);
		} else {
			if (!wasmApp.try_selecting(mouseX, mouseY)) {
				wasmApp.release_selection();
			}
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
