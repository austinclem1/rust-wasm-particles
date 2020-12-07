// Records the last 100 framerates and displays the current fps
// and the average fps of the last 100 frames

export class FramerateDisplay {
	constructor() {
		this.fpsLabel = document.getElementById("fps-label");
		this.frames = [];
	}

	update(delta) {
		const fps = (1 / delta) * 1000;
		this.frames.push(fps);
		if (this.frames.length > 100) {
			this.frames.shift();
		}

		let min = Infinity;
		let sum = 0;
		for (let i = 0; i < this.frames.length; i++) {
			sum += this.frames[i];
			min = Math.min(this.frames[i], min);
		}
		let mean = sum / this.frames.length;

		this.fpsLabel.innerText = `
Frames Per Second
Average of Last 100: ${Math.round(mean)}
Minimum of Last 100: ${Math.round(min)}
`.trim();
	}
}
