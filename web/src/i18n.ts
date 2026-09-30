const en = {
    title: 'Zone editor · Road anomaly detector',
    language: 'Language',
    connecting: 'Connecting',
    configurator: 'Configurator',
    openApiDescription: 'OpenAPI description',
    heading: 'Define your',
    headingAccent: 'zones.',
    subtitle: 'Mark the areas of the road where events should be detected.',
    loading: 'Loading zones',
    reload: 'Reload',
    apply: 'Apply',
    save: 'Save to TOML',
    detectionAreas: 'DETECTION AREAS',
    zones: 'Zones',
    draw: 'Draw a zone',
    wholeFrame: 'Whole frame',
    defaultArea: 'Default detection area',
    selectedZone: 'SELECTED ZONE',
    identifier: 'Identifier',
    outlineColor: 'Outline color',
    edit: 'Edit vertices',
    done: 'Done',
    delete: 'Delete',
    deleteLabel: 'Delete selected zone',
    fourPoints: 'Four points along the boundary.',
    oneZone: 'One zone for the area you need.',
    waiting: 'WAITING',
    sourceFrame: 'Source frame',
    noDimensions: 'no data',
    waitingFrame: 'Waiting for the first frame',
    waitingDescription: 'Video will appear when the detector starts processing.',
    previewAlt: 'Video preview for editing detection zones',
    canvasLabel: 'Zone editor. Click four points on the frame to create a zone.',
    canvasHint: 'Select a zone or draw a new one.',
    cancel: 'Cancel · Esc',
    rightClick: 'Right-click: edit vertices',
    connectingDetector: 'Connecting to the detector',
    frames: 'PROCESSED FRAMES',
    footer: 'Zone changes take effect without restarting the detector.',
    palette: 'Palette inspired by Gleam',
    saving: 'Saving…',
    dirty: 'You have unsaved draft changes',
    accepted: 'Accepted, but not saved to TOML',
    pending: 'Saved · applies on the next frame',
    saved: 'Zones saved',
    wholeFrameDescription: 'Events are detected across the frame, with a 5 px inset from each edge.',
    emptyZones: 'No zones. Event detection is disabled. Draw a zone or enable the whole frame.',
    firstPoint: 'Point A: click the first vertex, then three more along the boundary.',
    nextPoint: 'Point {point}: click the next vertex along the boundary.',
    zoneAdded: 'Zone added to your draft. Edit its vertices or save the result.',
    confirmWholeFrame: 'Replace the drawn zones with the whole-frame area?',
    confirmReload: 'Discard your local draft and load zones from the server?',
    zonesLoaded: 'Zones loaded from the server.',
    savedBackup: 'Zones saved to TOML. Backup: {backup}',
    appliedNotice: 'Zones accepted and will apply on the next frame. Click "Save to TOML" to keep them after a restart.',
    saveFailed: 'Zones were accepted by the detector, but saving TOML failed. ',
    previewUnavailable: 'Preview unavailable',
    previewRetry: 'Trying to reconnect. Your zone draft stays in this tab.',
    noPreview: 'NO PREVIEW',
    connected: 'Detector connected',
    noNewFrames: 'No new frames',
    waitingVideo: 'Waiting for video',
    live: 'LIVE',
    noFrames: 'NO FRAMES',
    offline: 'OFFLINE',
    disconnected: 'No connection',
    remoteChanged: 'A newer version of the zones is available on the server. Your draft is unchanged. Click "Reload" to load it.',
    conflict: 'Zones were changed by another client or in TOML. Your draft stays in this tab. Click "Reload" to load the current zones. If the file was edited, restart the detector first.',
    networkError: 'Cannot reach the server. Check the connection and try again.',
    timeout: 'The request timed out. Reload zones to check whether the operation was accepted before trying again.',
};

export type MessageKey = keyof typeof en;
export type Language = 'en' | 'ru';

const ru: Record<MessageKey, string> = {
    title: 'Редактор зон · Road anomaly detector',
    language: 'Язык',
    connecting: 'Подключение',
    configurator: 'Конфигуратор',
    openApiDescription: 'Описание OpenAPI',
    heading: 'Зоны',
    headingAccent: 'наблюдения.',
    subtitle: 'Отметьте участки дороги, в которых нужно фиксировать события.',
    loading: 'Загрузка зон',
    reload: 'Обновить',
    apply: 'Применить',
    save: 'Сохранить в TOML',
    detectionAreas: 'ОБЛАСТИ ДЕТЕКЦИИ',
    zones: 'Зоны',
    draw: 'Нарисовать зону',
    wholeFrame: 'Весь кадр',
    defaultArea: 'Область по умолчанию',
    selectedZone: 'ВЫБРАННАЯ ЗОНА',
    identifier: 'Идентификатор',
    outlineColor: 'Цвет контура',
    edit: 'Двигать вершины',
    done: 'Готово',
    delete: 'Удалить',
    deleteLabel: 'Удалить выбранную зону',
    fourPoints: 'Четыре точки по контуру.',
    oneZone: 'Одна зона для нужного участка.',
    waiting: 'ОЖИДАНИЕ',
    sourceFrame: 'Исходный кадр',
    noDimensions: 'нет данных',
    waitingFrame: 'Ждём первый кадр',
    waitingDescription: 'Видео появится, когда детектор начнёт обработку.',
    previewAlt: 'Видеопоток для настройки зон',
    canvasLabel: 'Редактор зон. Создайте зону четырьмя нажатиями на кадр.',
    canvasHint: 'Выберите зону или нарисуйте новую.',
    cancel: 'Отменить · Esc',
    rightClick: 'ПКМ: вершины',
    connectingDetector: 'Подключение к детектору',
    frames: 'ОБРАБОТАНО КАДРОВ',
    footer: 'Изменения зон применяются без перезапуска детектора.',
    palette: 'Палитра по мотивам Gleam',
    saving: 'Сохранение…',
    dirty: 'Есть изменения в черновике',
    accepted: 'Принято, но не сохранено в TOML',
    pending: 'Сохранено · применение на следующем кадре',
    saved: 'Зоны сохранены',
    wholeFrameDescription: 'События фиксируются по всему кадру с отступом 5 px от края.',
    emptyZones: 'Зон нет. Фиксация событий отключена. Нарисуйте зону или включите весь кадр.',
    firstPoint: 'Точка A: нажмите на первую вершину. Затем ещё три точки по контуру.',
    nextPoint: 'Точка {point}: нажмите на следующую вершину по контуру.',
    zoneAdded: 'Зона добавлена в черновик. Можно изменить вершины или сохранить результат.',
    confirmWholeFrame: 'Заменить нарисованные зоны областью всего кадра?',
    confirmReload: 'Отменить локальный черновик и загрузить зоны с сервера?',
    zonesLoaded: 'Зоны загружены с сервера.',
    savedBackup: 'Зоны сохранены в TOML. Резервная копия: {backup}',
    appliedNotice: 'Зоны приняты и будут применены на следующем кадре. Для сохранения после перезапуска нажмите "Сохранить в TOML".',
    saveFailed: 'Зоны приняты детектором, но сохранить TOML не удалось. ',
    previewUnavailable: 'Превью недоступно',
    previewRetry: 'Пробуем подключиться снова. Черновик зон остаётся в этой вкладке.',
    noPreview: 'НЕТ ПРЕВЬЮ',
    connected: 'Детектор подключён',
    noNewFrames: 'Нет новых кадров',
    waitingVideo: 'Ожидание видео',
    live: 'В ЭФИРЕ',
    noFrames: 'НЕТ КАДРОВ',
    offline: 'НЕ В СЕТИ',
    disconnected: 'Нет соединения',
    remoteChanged: 'На сервере появилась другая версия зон. Ваш черновик не изменён. Нажмите "Обновить", чтобы загрузить новую версию.',
    conflict: 'Зоны уже изменены в другом клиенте или в TOML. Черновик сохранён в этой вкладке. Нажмите "Обновить", чтобы загрузить актуальные зоны. Если изменён файл, сначала перезапустите детектор.',
    networkError: 'Нет связи с сервером. Проверьте соединение и попробуйте снова.',
    timeout: 'Время ожидания истекло. Перед повторной попыткой обновите зоны, чтобы проверить, была ли операция принята.',
};

let language: Language = 'en';
try {
    if (localStorage.getItem('road-anomaly-language') === 'ru') language = 'ru';
} catch {
    // Language switching also works when browser storage is unavailable.
}

export function getLanguage(): Language { return language; }

export function setLanguage(value: Language) {
    language = value;
    try { localStorage.setItem('road-anomaly-language', value); } catch {}
}

export function t(key: MessageKey, values: Record<string, string> = {}): string {
    return (language === 'ru' ? ru[key] : en[key]).replace(/\{(\w+)\}/g, (match, name: string) => values[name] ?? match);
}

export function translatePage() {
    document.documentElement.lang = language;
    document.title = t('title');
    for (const [attribute, target] of [['data-i18n', 'text'], ['data-i18n-label', 'aria-label'], ['data-i18n-alt', 'alt']]) {
        document.querySelectorAll<HTMLElement>(`[${attribute}]`).forEach(element => {
            const key = element.getAttribute(attribute) as MessageKey;
            if (!(key in en)) throw new Error(`Unknown translation: ${key}`);
            if (target === 'text') element.textContent = t(key);
            else element.setAttribute(target, t(key));
        });
    }
}

export class ApiError extends Error {
    constructor(public status: number, message: string) { super(message); }
}

export function describeError(error: unknown): string {
    if (error instanceof ApiError && error.status === 409) return t('conflict');
    if (error instanceof DOMException && (error.name === 'TimeoutError' || error.name === 'AbortError')) return t('timeout');
    if (error instanceof TypeError) return t('networkError');
    const message = error instanceof Error ? error.message : String(error);
    if (language === 'en') return message;
    const errors: Record<string, string> = {
        'whole_frame requires an empty zones list': 'Режим всего кадра требует пустого списка зон',
        'Zone not found': 'Зона не найдена',
        'At most 256 zones are supported': 'Поддерживается не более 256 зон',
        'Zone IDs must be unique, nonempty and at most 128 bytes long': 'Идентификаторы зон должны быть уникальными, непустыми и не длиннее 128 байт',
    };
    if (errors[message]) return errors[message];
    return message
        .replace(/^Zone (.*): RGB values must be between 0 and 255$/, 'Зона $1: значения RGB должны быть от 0 до 255')
        .replace(/^Zone (.*): coordinates must be inside the original frame$/, 'Зона $1: координаты должны находиться внутри исходного кадра')
        .replace(/^Zone (.*): all four vertices must be distinct$/, 'Зона $1: все четыре вершины должны быть различными')
        .replace(/^Zone (.*): polygon must have an area and no intersecting edges$/, 'Зона $1: площадь должна быть ненулевой, рёбра не должны пересекаться')
        .replace("Can't save configuration:", 'Не удалось сохранить конфигурацию:')
        .replace('The configuration directory must be writable', 'Каталог конфигурации должен быть доступен для записи');
}
