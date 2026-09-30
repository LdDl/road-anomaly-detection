import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { setImmediate } from 'node:timers/promises';
import test from 'node:test';
import ts from 'typescript';

const source = await readFile(new URL('../src/mjpeg_stream.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } });
const { MjpegParser, playMjpeg } = await import(`data:text/javascript;base64,${Buffer.from(compiled.outputText).toString('base64')}`);

function frame(bytes) {
    return Buffer.concat([Buffer.from(`--frame\r\nContent-Type: image/jpeg\r\nContent-Length: ${bytes.length}\r\n\r\n`), Buffer.from(bytes), Buffer.from('\r\n')]);
}

test('accepts every possible split across multipart headers and binary data', () => {
    const bytes = [255, 216, 0, 13, 10, 255, 217];
    const input = frame(bytes);
    for (let split = 0; split <= input.length; split++) {
        const parser = new MjpegParser('frame');
        const first = parser.push(input.subarray(0, split));
        const second = parser.push(input.subarray(split));
        assert.deepEqual(Array.from(second ?? first), bytes);
    }
    const parser = new MjpegParser('frame');
    const output = [];
    for (const byte of Buffer.concat([input, input])) {
        const result = parser.push(new Uint8Array([byte]));
        if (result) output.push(Array.from(result));
    }
    assert.deepEqual(output, [bytes, bytes]);
});

test('retains the newest complete frame and preserves an incomplete next frame', () => {
    const parser = new MjpegParser('frame');
    const next = frame([3, 4, 5]);
    assert.deepEqual(Array.from(parser.push(Buffer.concat([frame([1]), frame([2]), next.subarray(0, -3)]))), [2]);
    assert.deepEqual(Array.from(parser.push(next.subarray(-3))), [3, 4, 5]);
});

test('rejects malformed headers and excessive frame sizes before allocating the body', () => {
    for (const length of ['', '0', '-1', '8388609', '999999999999999999999']) {
        const parser = new MjpegParser('frame');
        assert.throws(() => parser.push(Buffer.from(`--frame\r\nContent-Length: ${length}\r\n\r\n`)), /length/);
    }
    assert.throws(() => new MjpegParser('other').push(frame([1])), /boundary/);
    assert.throws(() => new MjpegParser('frame').push(new Uint8Array(8193)), /header/);
});

function environment(t) {
    const jobs = new Map();
    let nextTimer = 0;
    let now = 0;
    for (const name of ['setTimeout', 'setInterval']) {
        t.mock.method(globalThis, name, (callback, delay) => {
            const id = ++nextTimer;
            jobs.set(id, { callback, delay, repeat: name === 'setInterval' });
            return id;
        });
    }
    for (const name of ['clearTimeout', 'clearInterval']) t.mock.method(globalThis, name, id => jobs.delete(id));
    t.mock.method(performance, 'now', () => now);
    const decodes = [];
    const originalImage = globalThis.Image;
    globalThis.Image = class {
        src = '';
        removeAttribute() { this.src = ''; }
        decode() { return new Promise((resolve, reject) => decodes.push({ url: this.src, resolve, reject })); }
    };
    t.after(() => {
        if (originalImage) globalThis.Image = originalImage;
        else delete globalThis.Image;
    });
    const blobs = new Map();
    let nextUrl = 0;
    t.mock.method(URL, 'createObjectURL', blob => {
        const url = `blob:${++nextUrl}`;
        blobs.set(url, blob);
        return url;
    });
    t.mock.method(URL, 'revokeObjectURL', url => blobs.delete(url));
    const connections = [];
    t.mock.method(globalThis, 'fetch', async (_url, { signal }) => {
        let input;
        const body = new ReadableStream({ start(controller) { input = controller; } });
        signal.addEventListener('abort', () => {
            try { input.error(new Error('Aborted')); } catch {}
        }, { once: true });
        connections.push({ input, signal, body });
        return new Response(body, { headers: { 'Content-Type': 'multipart/x-mixed-replace; boundary="frame"' } });
    });
    const target = { src: '', removeAttribute() { this.src = ''; } };
    const states = [];
    const player = playMjpeg('/live_streaming', target, state => states.push(state));
    t.after(() => player.stop());
    return {
        connections, decodes, blobs, target, states, player, jobs,
        fire(repeat, time) {
            now = time;
            const entry = [...jobs].find(([, job]) => job.repeat === repeat);
            assert.ok(entry, 'Expected a scheduled timer');
            const [id, job] = entry;
            if (!repeat) jobs.delete(id);
            job.callback();
            return job.delay;
        },
    };
}

test('reports playing only after decoding and skips queued frames under decode pressure', async t => {
    const env = environment(t);
    await setImmediate();
    assert.deepEqual(env.states, ['connecting']);
    env.connections[0].input.enqueue(frame([1]));
    await setImmediate();
    env.connections[0].input.enqueue(frame([2]));
    env.connections[0].input.enqueue(frame([3]));
    await setImmediate();
    assert.equal(env.decodes.length, 1);
    assert.deepEqual(env.states, ['connecting']);
    env.decodes[0].resolve();
    await setImmediate();
    assert.deepEqual(env.states, ['connecting', 'playing']);
    assert.deepEqual(Array.from(new Uint8Array(await env.blobs.get(env.decodes[1].url).arrayBuffer())), [3]);
    env.decodes[1].resolve();
    await setImmediate();
    assert.equal(env.blobs.size, 1);
    env.player.stop();
    await setImmediate();
    assert.equal(env.blobs.size, 0);
    assert.equal(env.jobs.size, 0);
    assert.equal(env.connections[0].body.locked, false);
});

test('watchdog reconnects a stuck decoder and rejects its late result after stopping', async t => {
    const env = environment(t);
    await setImmediate();
    env.connections[0].input.enqueue(frame([1]));
    await setImmediate();
    env.fire(true, 6000);
    await setImmediate();
    assert.equal(env.connections[0].signal.aborted, true);
    assert.deepEqual(env.states, ['connecting', 'stalled']);
    assert.equal(env.blobs.size, 0);
    assert.equal(env.fire(false, 6500), 500);
    await setImmediate();
    assert.equal(env.connections.length, 2);
    env.player.stop();
    env.decodes[0].resolve();
    await setImmediate();
    assert.equal(env.target.src, '');
    assert.equal(env.jobs.size, 0);
    assert.deepEqual(env.states, ['connecting', 'stalled']);
});

test('EOF reconnects with backoff even if HTTP headers were received successfully', async t => {
    const env = environment(t);
    await setImmediate();
    for (const delay of [500, 1000, 2000, 4000, 5000, 5000]) {
        env.connections.at(-1).input.close();
        await setImmediate();
        assert.equal(env.connections.at(-1).body.locked, false);
        assert.equal(env.fire(false, 0), delay);
        await setImmediate();
    }
    env.player.stop();
    await setImmediate();
    assert.equal(env.jobs.size, 0);
});
