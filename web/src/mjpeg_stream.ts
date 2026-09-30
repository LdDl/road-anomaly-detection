export type StreamState = 'connecting' | 'playing' | 'stalled';

export interface MjpegPlayer {
    stop(): void;
}

const HEADER_LIMIT = 8192;
const FRAME_LIMIT = 8 * 1024 * 1024;
const STALL_MS = 6000;

// The detector includes Content-Length in every multipart frame.
export class MjpegParser {
    private header: number[] = [];
    private frame: Uint8Array<ArrayBuffer> | undefined;
    private offset = 0;

    constructor(private boundary: string) {}

    push(chunk: Uint8Array): Uint8Array<ArrayBuffer> | undefined {
        let latest: Uint8Array<ArrayBuffer> | undefined;
        let at = 0;
        while (at < chunk.length) {
            if (this.frame) {
                const count = Math.min(chunk.length - at, this.frame.length - this.offset);
                this.frame.set(chunk.subarray(at, at + count), this.offset);
                this.offset += count;
                at += count;
                if (this.offset === this.frame.length) {
                    latest = this.frame;
                    this.frame = undefined;
                    this.offset = 0;
                }
                continue;
            }

            this.header.push(chunk[at++]);
            if (this.header.length > HEADER_LIMIT) throw new Error('MJPEG header is too large');
            const end = this.header.length;
            if (end < 4 || this.header[end - 4] !== 13 || this.header[end - 3] !== 10 || this.header[end - 2] !== 13 || this.header[end - 1] !== 10) continue;
            const header = new TextDecoder().decode(new Uint8Array(this.header)).replace(/^\r\n/, '');
            if (!header.startsWith(`--${this.boundary}\r\n`)) throw new Error('Invalid MJPEG boundary');
            const length = Number(/^Content-Length:\s*(\d+)\s*$/im.exec(header)?.[1]);
            if (!Number.isSafeInteger(length) || length <= 0 || length > FRAME_LIMIT) throw new Error('Invalid MJPEG frame length');
            this.frame = new Uint8Array(length);
            this.header = [];
        }
        return latest;
    }
}

function decodeFrame(url: string, signal: AbortSignal): Promise<void> {
    return new Promise((resolve, reject) => {
        const image = new Image();
        const finish = (error?: unknown) => {
            signal.removeEventListener('abort', abort);
            image.removeAttribute('src');
            if (error) reject(error);
            else resolve();
        };
        const abort = () => finish(new Error('MJPEG decoding aborted'));
        if (signal.aborted) { abort(); return; }
        signal.addEventListener('abort', abort, { once: true });
        image.src = url;
        image.decode().then(() => finish(), finish);
    });
}

// Keep one frame being decoded and only the newest frame waiting behind it.
export function playMjpeg(url: string, target: HTMLImageElement, onState: (state: StreamState) => void): MjpegPlayer {
    let stopped = false;
    let controller: AbortController | undefined;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    let retryDelay = 500;
    let shownUrl: string | undefined;
    let state: StreamState = 'connecting';
    onState(state);

    function report(next: StreamState) {
        if (stopped || next === state) return;
        state = next;
        onState(next);
    }

    async function connect() {
        if (stopped) return;
        const connection = new AbortController();
        controller = connection;
        let lastFrameAt = performance.now();
        let pending: Uint8Array<ArrayBuffer> | undefined;
        let decoding = false;
        let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
        const watchdog = setInterval(() => {
            if (performance.now() - lastFrameAt >= STALL_MS) connection.abort();
        }, 1000);

        async function displayLatest() {
            if (decoding) return;
            decoding = true;
            try {
                while (pending && !connection.signal.aborted) {
                    const objectUrl = URL.createObjectURL(new Blob([pending], { type: 'image/jpeg' }));
                    pending = undefined;
                    try {
                        await decodeFrame(objectUrl, connection.signal);
                        if (connection.signal.aborted) break;
                        const previous = shownUrl;
                        target.src = objectUrl;
                        shownUrl = objectUrl;
                        if (previous) URL.revokeObjectURL(previous);
                        lastFrameAt = performance.now();
                        retryDelay = 500;
                        report('playing');
                    } finally {
                        if (shownUrl !== objectUrl) URL.revokeObjectURL(objectUrl);
                    }
                }
            } catch {
                connection.abort();
            } finally {
                decoding = false;
            }
        }

        try {
            const response = await fetch(url, { signal: connection.signal, cache: 'no-store' });
            if (!response.ok || !response.body) throw new Error(`MJPEG HTTP ${response.status}`);
            reader = response.body.getReader();
            const contentType = response.headers.get('content-type') ?? '';
            const match = /boundary\s*=\s*(?:"([^"\r\n]+)"|([^;\s]+))/i.exec(contentType);
            if (!/^multipart\/x-mixed-replace\b/i.test(contentType) || !match) throw new Error('Invalid MJPEG content type');
            const parser = new MjpegParser(match[1] ?? match[2]);
            while (!connection.signal.aborted) {
                const { done, value } = await reader.read();
                if (done) break;
                pending = parser.push(value) ?? pending;
                void displayLatest();
            }
        } catch {
            // Network failures, incomplete streams and decode errors all reconnect.
        } finally {
            clearInterval(watchdog);
            connection.abort();
            pending = undefined;
            if (reader) {
                await reader.cancel().catch(() => {});
                reader.releaseLock();
            }
            if (!stopped) {
                report('stalled');
                retryTimer = setTimeout(() => void connect(), retryDelay);
                retryDelay = Math.min(retryDelay * 2, 5000);
            }
        }
    }

    void connect();
    return {
        stop() {
            stopped = true;
            clearTimeout(retryTimer);
            controller?.abort();
            target.removeAttribute('src');
            if (shownUrl) URL.revokeObjectURL(shownUrl);
            shownUrl = undefined;
        },
    };
}
