// Adapted from face_slop, originally based on rust-road-traffic-ui.
import { Polygon, FabricText, Shadow, util, type Canvas } from 'fabric';
import { createVertexControls } from './custom_control';

const VERTEX_LABELS = ['A', 'B', 'C', 'D'];

export interface CustomPolygonOptions {
	id: string;
	points: { x: number; y: number }[];
	color: [number, number, number];
	canvas: Canvas;
	onModified?: (id: string, newPoints: { x: number; y: number }[]) => void;
}

export class CustomPolygon {
	polygon: Polygon;
	labels: FabricText[];
	canvas: Canvas;
	id: string;
	color: [number, number, number];
	isEditing: boolean = false;
	onModified?: (id: string, newPoints: { x: number; y: number }[]) => void;
	private originalPoints: { x: number; y: number }[] = [];

	constructor(options: CustomPolygonOptions) {
		this.id = options.id;
		this.canvas = options.canvas;
		this.color = options.color;
		this.onModified = options.onModified;

		const colorStr = `rgb(${options.color[0]}, ${options.color[1]}, ${options.color[2]})`;

		// Create polygon - hasControls: false by default, enabled only in edit mode
		this.polygon = new Polygon(options.points, {
			fill: `rgba(${options.color.join(',')}, 0.12)`,
			originX: 'center',
			originY: 'center',
			left: (Math.min(...options.points.map(p => p.x)) + Math.max(...options.points.map(p => p.x))) / 2,
			top: (Math.min(...options.points.map(p => p.y)) + Math.max(...options.points.map(p => p.y))) / 2,
			stroke: colorStr,
			strokeWidth: 3,
			objectCaching: false,
			selectable: true,
			hasControls: false,
			hasBorders: true,
			lockMovementX: true,
			lockMovementY: true,
			lockRotation: true,
			lockScalingX: true,
			lockScalingY: true,
			cornerColor: '#ffaff3',
			cornerSize: 15,
			cornerStyle: 'circle',
			transparentCorners: false
		});

		// Add custom vertex controls (hidden by default via setControlVisible)
		const vertexControls = createVertexControls();
		Object.assign(this.polygon.controls, vertexControls);

		// Explicitly hide all vertex controls using setControlVisible
		for (let i = 0; i < 4; i++) {
			this.polygon.setControlVisible(`vertex_${i}`, false);
		}

		// Store reference to CustomPolygon on the fabric object
		(this.polygon as any).customPolygon = this;

		// Create labels
		const textShadow = new Shadow({
			color: 'rgba(255, 255, 255, 0.8)',
			blur: 8
		});

		this.labels = options.points.map((pt, idx) => {
			const label = new FabricText(VERTEX_LABELS[idx] || `${idx + 1}`, {
				left: pt.x - 8,
				top: pt.y - 24,
				fontSize: 18,
				fontFamily: 'monospace',
				fill: colorStr,
				shadow: textShadow,
				stroke: 'black',
				strokeWidth: 0.5,
				selectable: false,
				evented: false
			});
			return label;
		});

		// Add to canvas
		this.canvas.add(this.polygon);
		this.labels.forEach((label) => this.canvas.add(label));

		// Setup event handlers
		this.setupEventHandlers();
	}

	private setupEventHandlers() {
		// Right-click to toggle edit mode
		this.polygon.on('mousedown', (opt: any) => {
			if (opt.e.button === 2) {
				// Right click
				opt.e.preventDefault();
				opt.e.stopPropagation();
				this.toggleEditMode();
			}
		});

		// Update labels on modification (notification happens in exitEditMode)
		this.polygon.on('modified', () => {
			this.updateLabelsPosition();
		});

		// Update labels during object moving/scaling
		this.polygon.on('moving', () => this.updateLabelsPosition());
		this.polygon.on('scaling', () => this.updateLabelsPosition());
	}

	toggleEditMode() {
		if (this.isEditing) {
			this.exitEditMode();
		} else {
			this.enterEditMode();
		}
	}

	enterEditMode() {
		if (this.isEditing) return;
		this.isEditing = true;

		// Save original points for comparison
		this.originalPoints = this.getPoints();

		// Enable controls and hide borders
		this.polygon.set({
			hasControls: true,
			hasBorders: false
		});

		// Hide standard corner controls
		const standardControls = ['tl', 'tr', 'bl', 'br', 'ml', 'mr', 'mt', 'mb', 'mtr'];
		standardControls.forEach((name) => {
			this.polygon.setControlVisible(name, false);
		});

		// Show vertex controls
		this.showVertexControls();

		this.canvas.setActiveObject(this.polygon);
		this.canvas.requestRenderAll();
	}

	exitEditMode() {
		if (!this.isEditing) return;
		this.isEditing = false;

		// Hide vertex controls
		this.hideVertexControls();

		// Restore standard controls visibility (but keep them locked)
		this.polygon.set({
			hasControls: false,
			hasBorders: true
		});

		this.updateLabelsPosition();

		// Only notify if points actually changed
		if (this.onModified) {
			const currentPoints = this.getPoints();
			const hasChanged = this.pointsChanged(this.originalPoints, currentPoints);
			if (hasChanged) {
				this.onModified(this.id, currentPoints);
			}
		}

		this.canvas.discardActiveObject();
		this.canvas.requestRenderAll();
	}

	private pointsChanged(
		original: { x: number; y: number }[],
		current: { x: number; y: number }[]
	): boolean {
		if (original.length !== current.length) return true;

		const threshold = 0.01;
		for (let i = 0; i < original.length; i++) {
			const dx = Math.abs(original[i].x - current[i].x);
			const dy = Math.abs(original[i].y - current[i].y);
			if (dx > threshold || dy > threshold) {
				return true;
			}
		}
		return false;
	}

	private showVertexControls() {
		for (let i = 0; i < 4; i++) {
			this.polygon.setControlVisible(`vertex_${i}`, true);
		}
	}

	private hideVertexControls() {
		for (let i = 0; i < 4; i++) {
			this.polygon.setControlVisible(`vertex_${i}`, false);
		}
	}

	updateLabelsPosition() {
		const points = this.polygon.points!;
		const matrix = this.polygon.calcTransformMatrix();

		points.forEach((point, idx) => {
			if (this.labels[idx]) {
				const transformed = util.transformPoint(
					{
						x: point.x - this.polygon.pathOffset.x,
						y: point.y - this.polygon.pathOffset.y
					} as any,
					matrix
				);
				this.labels[idx].set({
					left: transformed.x - 8,
					top: transformed.y - 24
				});
			}
		});
	}

	getPoints(): { x: number; y: number }[] {
		const points = this.polygon.points!;
		const matrix = this.polygon.calcTransformMatrix();

		return points.map((point) => {
			const transformed = util.transformPoint(
				{
					x: point.x - this.polygon.pathOffset.x,
					y: point.y - this.polygon.pathOffset.y
				} as any,
				matrix
			);
			return { x: transformed.x, y: transformed.y };
		});
	}

	remove() {
		this.canvas.remove(this.polygon);
		this.labels.forEach((label) => this.canvas.remove(label));
	}
}
