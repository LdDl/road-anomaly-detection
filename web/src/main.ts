import { Canvas, Circle, FabricObject, FabricText, Line, type Point } from 'fabric';
import { CustomPolygon } from './canvas/custom_polygon';
import { ApiError, describeError, getLanguage, t, translatePage } from './i18n';
import { createLanguagePicker } from './language_picker';
import { playMjpeg, type MjpegPlayer, type StreamState } from './mjpeg_stream';
import './style.css';

type Coordinates = [number, number];
type Color = [number, number, number];
interface Zone { id: string; geometry: Coordinates[]; color_rgb: Color | null }
interface ZonesResponse { zones: Zone[]; whole_frame: boolean; revision: number; saved_revision: number }
interface Health {
    status: string;
    equipment_id: string;
    revision: number;
    applied_revision: number;
    saved_revision: number;
    video: { width: number; height: number; fps: number; frames: number };
}

function element<T extends HTMLElement>(id: string): T {
    const value = document.getElementById(id);
    if (!value) throw new Error(`Missing element: ${id}`);
    return value as T;
}

const ui = {
    add: element<HTMLButtonElement>('add-zone'),
    apply: element<HTMLButtonElement>('apply'),
    save: element<HTMLButtonElement>('save'),
    reload: element<HTMLButtonElement>('reload'),
    whole: element<HTMLInputElement>('whole-frame'),
    list: element('zone-list'),
    editor: element('zone-editor'),
    id: element<HTMLInputElement>('zone-id'),
    color: element<HTMLInputElement>('zone-color'),
    edit: element<HTMLButtonElement>('edit-zone'),
    delete: element<HTMLButtonElement>('delete-zone'),
    stage: element('stage'),
    well: element('video-well'),
    stream: element<HTMLImageElement>('stream'),
    hint: element('canvas-hint'),
    cancel: element<HTMLButtonElement>('cancel-draw'),
    notice: element('notice'),
};

const layout = {
    main: document.querySelector<HTMLElement>('main')!,
    workspace: document.querySelector<HTMLElement>('.workspace')!,
    actions: document.querySelector<HTMLElement>('.save-actions')!,
    footer: document.querySelector<HTMLElement>('.page-footer')!,
    video: document.querySelector<HTMLElement>('.video-panel')!,
    videoHeading: document.querySelector<HTMLElement>('.video-heading')!,
    videoToolbar: document.querySelector<HTMLElement>('.canvas-toolbar')!,
    videoFooter: document.querySelector<HTMLElement>('.video-footer')!,
};
let workspaceMaxHeight: number | undefined;

FabricObject.ownDefaults.originX = 'left';
FabricObject.ownDefaults.originY = 'top';
const canvas = new Canvas('zone-canvas', {
    selection: false,
    fireRightClick: true,
    stopContextMenu: true,
    preserveObjectStacking: true,
});

let zones: Zone[] = [];
let wholeFrame = false;
let baseline = '';
let revision = 0;
let savedRevision = 0;
let appliedRevision = 0;
let loaded = false;
let online = false;
let busy = false;
let selected: number | null = null;
let width = 0;
let height = 0;
let drawing = false;
let draftPoints: Coordinates[] = [];
let polygons: CustomPolygon[] = [];
let helpers: FabricObject[] = [];
let rebuilding = false;
let stream: MjpegPlayer | undefined;
let streamReady = false;
let previewFailed = false;
let lastHealth: Health | undefined;
let checkedConnection = false;
let noticeMessage: (() => string) | undefined;

const colors: Color[] = [[255, 175, 243], [166, 240, 252], [255, 251, 232], [188, 233, 171]];
const clone = <T>(value: T): T => structuredClone(value);
const content = () => JSON.stringify({ zones, whole_frame: wholeFrame });
const dirty = () => loaded && content() !== baseline;
const colorHex = (color: Color | null) => `#${(color ?? [0, 0, 0]).map(value => value.toString(16).padStart(2, '0')).join('')}`;

function notice(message: () => string, error = false) {
    noticeMessage = message;
    ui.notice.textContent = message();
    ui.notice.hidden = !ui.notice.textContent;
    ui.notice.classList.toggle('error', error);
}

async function request<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(path, {
        method: body === undefined ? 'GET' : 'POST',
        headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(15000),
        cache: 'no-store',
    });
    if (!response.ok) {
        const result = await response.json().catch(() => ({}));
        throw new ApiError(response.status, result.error ?? `HTTP ${response.status}`);
    }
    return response.json() as Promise<T>;
}

function updateActions() {
    const blocked = !loaded || !online || busy;
    ui.add.disabled = blocked || !width || !height || !streamReady || drawing;
    ui.whole.disabled = blocked || drawing;
    ui.apply.disabled = blocked || drawing || (!dirty() && !polygons.some(p => p.isEditing));
    ui.save.disabled = blocked || drawing;
    ui.reload.disabled = busy || !online;
    ui.id.disabled = blocked;
    ui.color.disabled = blocked;
    ui.edit.disabled = blocked || !streamReady || drawing;
    ui.delete.disabled = blocked || drawing;
    ui.cancel.hidden = !drawing;
    canvas.skipTargetFind = blocked || drawing;
    canvas.upperCanvasEl.style.pointerEvents = blocked ? 'none' : 'auto';
    const state = element('save-state');
    state.classList.toggle('dirty', dirty() || revision !== savedRevision);
    state.textContent = busy ? t('saving') : !loaded ? t('loading') : dirty() ? t('dirty') : revision !== savedRevision ? t('accepted') : appliedRevision < revision ? t('pending') : t('saved');
}

function updateCoordinates() {
    const output = element('coordinates');
    output.replaceChildren();
    if (selected === null || !zones[selected]) return;
    zones[selected].geometry.forEach(([x, y], index) => {
        const value = document.createElement('span');
        const label = document.createElement('b');
        label.textContent = 'ABCD'[index];
        value.append(label, `${x}, ${y}`);
        output.append(value);
    });
}

function renderSidebar() {
    ui.whole.checked = wholeFrame;
    element('zone-count').textContent = String(zones.length);
    ui.list.replaceChildren();
    if (!zones.length) {
        const empty = document.createElement('p');
        empty.className = 'empty-list';
        empty.textContent = wholeFrame ? t('wholeFrameDescription') : t('emptyZones');
        ui.list.append(empty);
    }
    zones.forEach((zone, index) => {
        const row = document.createElement('button');
        row.className = `zone-row${selected === index ? ' active' : ''}`;
        row.disabled = busy || drawing;
        row.setAttribute('aria-pressed', String(selected === index));
        const swatch = document.createElement('span');
        swatch.className = 'swatch';
        swatch.style.setProperty('--zone-color', colorHex(zone.color_rgb));
        const name = document.createElement('span');
        name.className = 'zone-name';
        name.textContent = zone.id;
        row.title = zone.id;
        const number = document.createElement('span');
        number.className = 'zone-index';
        number.textContent = String(index + 1).padStart(2, '0');
        row.append(swatch, name, number);
        row.onclick = () => selectZone(index);
        ui.list.append(row);
    });
    const zone = selected === null ? undefined : zones[selected];
    ui.editor.hidden = !zone;
    if (zone) {
        if (document.activeElement !== ui.id) ui.id.value = zone.id;
        ui.color.value = colorHex(zone.color_rgb);
    }
    ui.edit.textContent = selected !== null && polygons[selected]?.isEditing ? t('done') : t('edit');
    updateCoordinates();
    updateActions();
}

function commitVertices() {
    polygons.forEach(polygon => polygon.exitEditMode());
    updateActions();
}

function selectZone(index: number) {
    commitVertices();
    selected = index;
    const polygon = polygons[index];
    if (polygon) canvas.setActiveObject(polygon.polygon);
    canvas.requestRenderAll();
    renderSidebar();
}

function redraw() {
    rebuilding = true;
    canvas.clear();
    polygons = [];
    helpers = [];
    if (width && height) {
        polygons = zones.map(zone => new CustomPolygon({
            id: zone.id,
            color: zone.color_rgb ?? [0, 0, 0],
            points: zone.geometry.map(([x, y]) => ({ x, y })),
            canvas,
            onModified: (_id, points) => {
                zone.geometry = points.map(point => [Math.round(point.x), Math.round(point.y)]);
                updateCoordinates();
                updateActions();
            },
        }));
        polygons.forEach((polygon, index) => {
            polygon.polygon.on('mousedown', () => {
                if (drawing) return;
                polygons.forEach(other => { if (other !== polygon) other.exitEditMode(); });
                selected = index;
                canvas.setActiveObject(polygon.polygon);
                renderSidebar();
            });
            polygon.polygon.on('modified', () => {
                zones[index].geometry = polygon.getPoints().map(point => [Math.round(point.x), Math.round(point.y)]);
                updateCoordinates();
                updateActions();
            });
        });
        if (wholeFrame && width > 10 && height > 10) {
            const whole = new CustomPolygon({ id: 'whole_image', color: [166, 240, 252], canvas, points: [{ x: 5, y: 5 }, { x: width - 5, y: 5 }, { x: width - 5, y: height - 5 }, { x: 5, y: height - 5 }] });
            whole.polygon.set({ selectable: false, evented: false, strokeDashArray: [8, 6] });
        }
    }
    if (selected !== null && polygons[selected] && !drawing) canvas.setActiveObject(polygons[selected].polygon);
    rebuilding = false;
    drawHelpers();
    canvas.requestRenderAll();
    renderSidebar();
}

function fitCanvas() {
    if (!width || !height) return;
    const style = getComputedStyle(ui.well);
    const available = ui.well.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    let availableHeight = 760;
    if (workspaceMaxHeight !== undefined) {
        const panelStyle = getComputedStyle(layout.video);
        const chromeHeight = layout.videoHeading.offsetHeight + layout.videoToolbar.offsetHeight + layout.videoFooter.offsetHeight;
        const borders = parseFloat(panelStyle.borderTopWidth) + parseFloat(panelStyle.borderBottomWidth) + parseFloat(style.borderTopWidth) + parseFloat(style.borderBottomWidth);
        availableHeight = Math.min(availableHeight, Math.max(1, workspaceMaxHeight - chromeHeight - borders - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom)));
    }
    const scale = Math.min(available / width, availableHeight / height);
    if (scale <= 0) return;
    if (Math.abs(canvas.getWidth() - width * scale) < 0.01 && Math.abs(canvas.getHeight() - height * scale) < 0.01 && Math.abs(canvas.getZoom() - scale) < 0.00001) return;
    ui.stage.style.width = `${width * scale}px`;
    ui.stage.style.height = `${height * scale}px`;
    canvas.setDimensions({ width: width * scale, height: height * scale });
    canvas.setViewportTransform([scale, 0, 0, scale, 0, 0]);
    canvas.calcOffset();
    canvas.requestRenderAll();
}

function drawHelpers(pointer?: Coordinates) {
    helpers.forEach(helper => canvas.remove(helper));
    helpers = [];
    const scale = canvas.getZoom();
    draftPoints.forEach(([x, y], index) => {
        if (index > 0) helpers.push(new Line([...draftPoints[index - 1], x, y], { stroke: '#ffaff3', strokeWidth: 2 / scale, selectable: false, evented: false }));
        helpers.push(new Circle({ left: x, top: y, originX: 'center', originY: 'center', radius: 4 / scale, fill: '#ffaff3', selectable: false, evented: false }));
        helpers.push(new FabricText('ABCD'[index], { left: x + 9 / scale, top: y - 22 / scale, fontSize: 14 / scale, fill: '#fffbe8', backgroundColor: '#292d3e', selectable: false, evented: false }));
    });
    if (pointer && draftPoints.length) helpers.push(new Line([...draftPoints[draftPoints.length - 1], ...pointer], { stroke: '#ffaff3', strokeWidth: 2 / scale, strokeDashArray: [6 / scale, 5 / scale], selectable: false, evented: false }));
    helpers.forEach(helper => canvas.add(helper));
    canvas.requestRenderAll();
}

function cancelDrawing() {
    drawing = false;
    draftPoints = [];
    canvas.defaultCursor = 'default';
    ui.hint.textContent = t('canvasHint');
    redraw();
}

function inFrame(point: Point): Coordinates {
    return [Math.max(0, Math.min(width - 1, Math.round(point.x))), Math.max(0, Math.min(height - 1, Math.round(point.y)))];
}

function nextId() {
    let index = 1;
    while (zones.some(zone => zone.id === `zone_${index}`)) index++;
    return `zone_${index}`;
}

canvas.on('mouse:down', event => {
    if (!drawing || !online || busy || ('button' in event.e && event.e.button === 2)) return;
    draftPoints.push(inFrame(event.scenePoint));
    drawHelpers();
    updateDrawingHint();
    if (draftPoints.length === 4) {
        zones.push({ id: nextId(), geometry: clone(draftPoints), color_rgb: clone(colors[zones.length % colors.length]) });
        wholeFrame = false;
        selected = zones.length - 1;
        cancelDrawing();
        notice(() => t('zoneAdded'));
    }
});
canvas.on('mouse:move', event => { if (drawing) drawHelpers(inFrame(event.scenePoint)); });
canvas.on('selection:cleared', () => {
    if (!rebuilding) updateActions();
});

ui.add.onclick = () => {
    commitVertices();
    selected = null;
    canvas.discardActiveObject();
    drawing = true;
    draftPoints = [];
    canvas.defaultCursor = 'crosshair';
    ui.hint.textContent = t('firstPoint');
    notice(() => '');
    renderSidebar();
};
ui.cancel.onclick = cancelDrawing;
ui.edit.onclick = () => {
    if (selected === null) return;
    polygons[selected]?.toggleEditMode();
    renderSidebar();
};
ui.delete.onclick = () => {
    if (selected === null) return;
    commitVertices();
    zones.splice(selected, 1);
    selected = null;
    redraw();
};
ui.id.oninput = () => {
    if (selected === null) return;
    zones[selected].id = ui.id.value;
    if (polygons[selected]) polygons[selected].id = ui.id.value;
    renderSidebar();
};
ui.color.oninput = () => {
    if (selected === null) return;
    commitVertices();
    zones[selected].color_rgb = [1, 3, 5].map(offset => parseInt(ui.color.value.slice(offset, offset + 2), 16)) as Color;
    redraw();
};
ui.whole.onchange = () => {
    if (ui.whole.checked && zones.length && !window.confirm(t('confirmWholeFrame'))) {
        ui.whole.checked = wholeFrame;
        return;
    }
    commitVertices();
    wholeFrame = ui.whole.checked;
    if (wholeFrame) zones = [];
    selected = null;
    redraw();
};

async function loadZones() {
    const result = await request<ZonesResponse>('/api/zones');
    zones = clone(result.zones);
    wholeFrame = result.whole_frame;
    revision = result.revision;
    savedRevision = result.saved_revision;
    baseline = content();
    loaded = true;
    selected = null;
    cancelDrawing();
}

ui.reload.onclick = async () => {
    commitVertices();
    if ((dirty() || drawing) && !window.confirm(t('confirmReload'))) return;
    busy = true;
    updateActions();
    try { await loadZones(); notice(() => t('zonesLoaded')); }
    catch (error) { notice(() => describeError(error), true); }
    finally { busy = false; renderSidebar(); }
};

async function submit(save: boolean) {
    commitVertices();
    busy = true;
    renderSidebar();
    let applied = false;
    try {
        const result = await request<ZonesResponse>('/api/zones', { zones, whole_frame: wholeFrame, revision });
        revision = result.revision;
        savedRevision = result.saved_revision;
        baseline = content();
        applied = true;
        if (save) {
            const result = await request<{ backup: string; revision: number }>('/api/mutations/save_config', { revision });
            savedRevision = result.revision;
            notice(() => t('savedBackup', { backup: result.backup }));
        } else {
            notice(() => t('appliedNotice'));
        }
    } catch (error) {
        notice(() => `${applied ? t('saveFailed') : ''}${describeError(error)}`, true);
    } finally { busy = false; renderSidebar(); }
}
ui.apply.onclick = () => void submit(false);
ui.save.onclick = () => void submit(true);

function streamStateChanged(state: StreamState) {
    streamReady = state === 'playing';
    previewFailed = state === 'stalled';
    ui.stage.hidden = !streamReady;
    element('video-placeholder').hidden = streamReady;
    if (streamReady) fitCanvas();
    updatePreviewText();
    updateStatus();
    updateActions();
}

function stopStream() {
    stream?.stop();
    stream = undefined;
    streamStateChanged('connecting');
}

function startStream() {
    if (!stream && !document.hidden) stream = playMjpeg('/live_streaming', ui.stream, streamStateChanged);
}

document.addEventListener('visibilitychange', () => {
    if (document.hidden) stopStream();
    else startStream();
});
window.addEventListener('pagehide', stopStream);
window.addEventListener('pageshow', startStream);

async function poll() {
    try {
        const health = await request<Health>('/api/health');
        online = true;
        checkedConnection = true;
        lastHealth = health;
        appliedRevision = health.applied_revision;
        if (loaded && health.revision === revision) savedRevision = health.saved_revision;
        updateStatus();
        if (health.video.width && health.video.height && (width !== health.video.width || height !== health.video.height)) {
            commitVertices();
            width = health.video.width;
            height = health.video.height;
            element('dimensions').textContent = `${width} × ${height} px`;
            fitCanvas();
            redraw();
        }
        if (!loaded && !busy) await loadZones();
        if (loaded && health.revision !== revision && !busy) notice(() => t('remoteChanged'), true);
    } catch {
        online = false;
        checkedConnection = true;
        updateStatus();
    } finally {
        updateActions();
        setTimeout(() => void poll(), 2000);
    }
}

function fitWorkspace() {
    if (window.matchMedia('(min-width: 761px)').matches) {
        const footerStyle = getComputedStyle(layout.footer);
        const mainStyle = getComputedStyle(layout.main);
        const above = layout.workspace.getBoundingClientRect().top - layout.actions.getBoundingClientRect().top;
        const below = layout.footer.offsetHeight + parseFloat(footerStyle.marginTop) + parseFloat(mainStyle.paddingBottom);
        // Reserve room for the actions and footer when the page is scrolled to the bottom.
        workspaceMaxHeight = Math.max(240, Math.floor((window.visualViewport?.height ?? window.innerHeight) - above - below - 16));
        layout.workspace.style.setProperty('--workspace-max-height', `${workspaceMaxHeight}px`);
    } else {
        workspaceMaxHeight = undefined;
        layout.workspace.style.removeProperty('--workspace-max-height');
    }
    fitCanvas();
    if (drawing) drawHelpers();
}

let layoutFrame = 0;
function scheduleLayout() {
    cancelAnimationFrame(layoutFrame);
    layoutFrame = requestAnimationFrame(fitWorkspace);
}

const layoutObserver = new ResizeObserver(scheduleLayout);
for (const target of [ui.well, ui.notice, layout.actions, layout.footer, layout.videoHeading, layout.videoToolbar, layout.videoFooter]) {
    layoutObserver.observe(target);
}
window.addEventListener('resize', scheduleLayout);
window.visualViewport?.addEventListener('resize', scheduleLayout);
window.addEventListener('keydown', event => {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || (event.target instanceof Element && event.target.closest('.language-picker'))) return;
    if (event.key === 'Escape') {
        if (drawing) cancelDrawing();
        else { commitVertices(); renderSidebar(); }
    }
    if (event.key === 'Delete' && selected !== null && !ui.delete.disabled) ui.delete.click();
});
window.addEventListener('beforeunload', event => {
    commitVertices();
    if (dirty() || revision !== savedRevision || draftPoints.length) event.preventDefault();
});
function updateDrawingHint() {
    ui.hint.textContent = !drawing ? t('canvasHint') : !draftPoints.length ? t('firstPoint') : t('nextPoint', { point: 'ABCD'[Math.min(draftPoints.length, 3)] });
}

function updatePreviewText() {
    element('preview-title').textContent = t(previewFailed ? 'previewUnavailable' : 'waitingFrame');
    element('preview-description').textContent = t(previewFailed ? 'previewRetry' : 'waitingDescription');
}

function updateStatus() {
    const live = online && lastHealth?.status === 'streaming';
    const stale = online && lastHealth?.status === 'stale';
    element('connection').textContent = !checkedConnection ? t('connecting') : !online ? t('disconnected') : live ? t('connected') : stale ? t('noNewFrames') : t('waitingVideo');
    element('status-dot').className = `dot${live ? ' online' : !online && checkedConnection ? ' offline' : ''}`;
    element('stream-badge').textContent = !online && checkedConnection ? t('offline') : previewFailed ? t('noPreview') : live && streamReady ? t('live') : stale ? t('noFrames') : t('waiting');
    element('stream-badge').classList.toggle('live', !!live && streamReady);
    element('equipment-id').textContent = lastHealth?.equipment_id ?? t('connectingDetector');
    element('frame-count').textContent = (lastHealth?.video.frames ?? 0).toLocaleString(getLanguage());
    element('dimensions').textContent = width && height ? `${width} × ${height} px` : t('noDimensions');
}

function refreshLanguage() {
    translatePage();
    languagePicker.sync();
    renderSidebar();
    updateDrawingHint();
    updatePreviewText();
    updateStatus();
    if (noticeMessage) ui.notice.textContent = noticeMessage();
}

const languagePicker = createLanguagePicker(document.querySelector<HTMLElement>('.language-picker')!, refreshLanguage);

refreshLanguage();
startStream();
void poll();
