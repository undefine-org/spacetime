// Minimal CDP client over Chrome's remote-debugging websocket — no puppeteer.
// Exports: launch({args, width, height}) → { send(method, params), close(), on(event, fn) }
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const CHROME = process.env.SPACETIME_CHROME || 'google-chrome-stable';

export async function launch({ width = 1920, height = 1080, gpu = 'swiftshader', extra = [] } = {}) {
  const profile = mkdtempSync(join(tmpdir(), 'promo-chrome-'));
  const gl = gpu === 'swiftshader'
    ? ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist']
    : gpu === 'vulkan'
      ? ['--use-gl=angle', '--use-angle=vulkan', '--ignore-gpu-blocklist', '--enable-features=Vulkan']
      : ['--use-gl=angle', '--use-angle=gl', '--ignore-gpu-blocklist'];
  const args = [
    '--headless=new', '--no-sandbox', '--disable-dev-shm-usage',
    '--remote-debugging-port=0', `--user-data-dir=${profile}`,
    `--window-size=${width},${height}`, '--hide-scrollbars',
    '--force-device-scale-factor=1', '--font-render-hinting=none',
    '--disable-lcd-text', '--enable-font-antialiasing',
    '--allow-file-access-from-files', '--autoplay-policy=no-user-gesture-required',
    ...gl, ...extra, 'about:blank',
  ];
  const proc = spawn(CHROME, args, { stdio: ['ignore', 'ignore', 'pipe'] });
  const wsUrl = await new Promise((res, rej) => {
    let buf = '';
    proc.stderr.on('data', (d) => {
      buf += d.toString();
      const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
      if (m) res(m[1]);
    });
    proc.on('exit', (c) => rej(new Error(`chrome exited ${c}\n${buf}`)));
    setTimeout(() => rej(new Error('chrome did not start\n' + buf)), 20000);
  });
  const port = new URL(wsUrl).port;
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = targets.find((t) => t.type === 'page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0;
  const pending = new Map();
  const listeners = new Map();
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { res, rej } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? rej(new Error(`${msg.error.message} ${msg.error.data ?? ''}`)) : res(msg.result);
    } else if (msg.method && listeners.has(msg.method)) {
      for (const fn of listeners.get(msg.method)) fn(msg.params);
    }
  };
  const send = (method, params = {}) =>
    new Promise((res, rej) => {
      const i = ++id;
      pending.set(i, { res, rej });
      ws.send(JSON.stringify({ id: i, method, params }));
    });
  const on = (m, fn) => listeners.set(m, [...(listeners.get(m) ?? []), fn]);
  const evaluate = async (expression, awaitPromise = true) => {
    const r = await send('Runtime.evaluate', { expression, awaitPromise, returnByValue: true });
    if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
    return r.result.value;
  };
  const close = () => {
    try { ws.close(); } catch {}
    proc.kill('SIGKILL');
    rmSync(profile, { recursive: true, force: true });
  };
  await send('Page.enable');
  await send('Runtime.enable');
  return { send, on, evaluate, close, port };
}
