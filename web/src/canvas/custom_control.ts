// Originally based on rust-road-traffic-ui. See the ref.: https://github.com/LdDl/rust-road-traffic-ui
import { Control, util, Point } from 'fabric';
import type { TPointerEvent, Transform, TMat2D } from 'fabric';
import type { Polygon } from 'fabric';

// Helper: get polygon size including stroke
function getObjectSizeWithStroke(object: Polygon): Point {
	let width = object.width;
	let height = object.height;
	// If width/height are not set, calculate from bounding rect
	if (!width || !height || isNaN(width) || isNaN(height)) {
		const boundingRect = object.getBoundingRect();
		width = boundingRect.width;
		height = boundingRect.height;
	}
	const strokeWidth = object.strokeWidth || 0;
	return new Point(width + strokeWidth, height + strokeWidth);
}

// Wrapper that keeps anchor point in place after dimension recalculation
export function anchorWrapper(anchorIndex: number, fn: typeof actionHandler) {
	return function (eventData: TPointerEvent, transform: Transform, x: number, y: number): boolean {
		const fabricObject = transform.target as Polygon;
		const points = fabricObject.points!;

		// Calculate absolute position of anchor point BEFORE modification
		const pt = new Point(
			points[anchorIndex].x - fabricObject.pathOffset.x,
			points[anchorIndex].y - fabricObject.pathOffset.y
		);
		const absolutePoint = pt.transform(fabricObject.calcTransformMatrix());

		// Perform the actual action (moves vertex)
		const actionPerformed = fn(eventData, transform, x, y);

		// After setDimensions(), pathOffset may have changed
		// Reposition polygon so anchor point stays at its original screen position
		const polygonBaseSize = getObjectSizeWithStroke(fabricObject);

		const newX = (points[anchorIndex].x - fabricObject.pathOffset.x) / polygonBaseSize.x;
		const newY = (points[anchorIndex].y - fabricObject.pathOffset.y) / polygonBaseSize.y;

		fabricObject.setPositionByOrigin(absolutePoint, newX + 0.5, newY + 0.5);

		// Force recalculation of object coordinates and re-render
		fabricObject.setCoords();
		if (fabricObject.canvas) {
			fabricObject.canvas.requestRenderAll();
		}

		(fabricObject as any).customPolygon?.updateLabelsPosition();
		return actionPerformed;
	};
}

// Position handler - calculates screen position of vertex control
export function polygonPositionHandler(
	this: Control & { pointIndex: number },
	_dim: Point,
	_finalMatrix: TMat2D,
	fabricObject: Polygon
): Point {
	const points = fabricObject.points!;
	const pointIndex = this.pointIndex;
	const point = points[pointIndex];

	if (!point) {
		return new Point(0, 0);
	}

	const x = point.x - fabricObject.pathOffset.x;
	const y = point.y - fabricObject.pathOffset.y;
	const pt = new Point(x, y);

	// Combine viewport transform with object transform
	const canvas = fabricObject.canvas;
	if (canvas) {
		const result = pt.transform(
			util.multiplyTransformMatrices(canvas.viewportTransform, fabricObject.calcTransformMatrix())
		);
		return result;
	}

	// Fallback if no canvas
	return pt.transform(fabricObject.calcTransformMatrix());
}

// Action handler - updates vertex position during drag
export function actionHandler(
	_eventData: TPointerEvent,
	transform: Transform,
	x: number,
	y: number
): boolean {
	const polygon = transform.target as Polygon;
	const points = polygon.points!;
	const currentControl = polygon.controls[transform.corner!] as Control & { pointIndex: number };
	const pointIndex = currentControl.pointIndex;

	// Convert screen coordinates to local polygon coordinates
	const pt = new Point(x, y);
	const mouseLocalPosition = pt.transform(util.invertTransform(polygon.calcTransformMatrix()));

	const polygonBaseSize = getObjectSizeWithStroke(polygon);
	const size = (polygon as any)._getTransformedDimensions(0, 0);

	// Calculate final point position with proper scaling
	const finalPointPosition = {
		x: (mouseLocalPosition.x * polygonBaseSize.x) / size.x + polygon.pathOffset.x,
		y: (mouseLocalPosition.y * polygonBaseSize.y) / size.y + polygon.pathOffset.y
	};

	points[pointIndex] = finalPointPosition;

	// Recalculate bounding box after point modification
	polygon.setDimensions();

	// Mark polygon as dirty to force re-render
	polygon.set({ dirty: true });

	// Update labels during drag (if CustomPolygon reference exists)
	const customPolygon = (polygon as any).customPolygon;
	if (customPolygon && typeof customPolygon.updateLabelsPosition === 'function') {
		customPolygon.updateLabelsPosition();
	}

	return true;
}

// Create vertex controls for a 4-point polygon
export function createVertexControls(): Record<string, Control> {
	const controls: Record<string, Control> = {};
	const numPoints = 4;

	for (let i = 0; i < numPoints; i++) {
		const lastControl = numPoints - 1;
		// Anchor to previous point (or last point if we're at index 0)
		const anchorIndex = i > 0 ? i - 1 : lastControl;

		controls[`vertex_${i}`] = new Control({
			positionHandler: polygonPositionHandler,
			actionHandler: anchorWrapper(anchorIndex, actionHandler),
			actionName: 'modifyPolygon',
			cursorStyle: 'move',
			// @ts-ignore - custom property for point index
			pointIndex: i,
			visible: false
		});
	}

	return controls;
}
