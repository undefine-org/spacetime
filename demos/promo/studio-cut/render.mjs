// Deterministic frame capture: seek → screenshot → ffmpeg.
//   node render.mjs                     full 30s @ 60fps → out/video.mp4 (+ audio mux → out/spacetime-promo-studio.mp4)
//   node render.mjs --stills 1,4.5,9    PNG stills at those seconds → out/stills/t-*.png
//   node render.mjs --fps 30 --range 17,24
import { launch } from './cdp.mjs';
import { start } from './serve.mjs';
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

const args = Object.fromEntries(process.argv.slice(2).map((a, i, all) => a.startsWith('--') ? [a.slice(2), all[i + 1]?.startsWith('--') || all[i + 1] === undefined ? true : all[i + 1]] : []).filter(Boolean));
const W = 1920, H = 1080, FPS = Number(args.fps ?? 60), DUR = 30;
const GPU = args.gpu ?? 'gl';
const OUT = resolve('out'); mkdirSync(OUT, { recursive: true });

const srv = await start();
const browser = await launch({ width: W, height: H, gpu: GPU });
await browser.send('Emulation.setDeviceMetricsOverride', { width: W, height: H, deviceScaleFactor: 1, mobile: false });
const loaded = new Promise((r) => browser.on('Page.loadEventFired', r));
await browser.send('Page.navigate', { url: `http://127.0.0.1:${srv.port}/demos/promo/studio-cut/index.html#capture` });
await loaded;
for (let i = 0; i < 200; i++) { if (await browser.evaluate('window.__ready === true')) break; await new Promise((r) => setTimeout(r, 100)); }
if (!(await browser.evaluate('window.__ready === true'))) throw new Error('page never became ready');
console.log('page ready · gpu:', await browser.evaluate(`(()=>{const c=document.createElement('canvas');const gl=c.getContext('webgl2');const d=gl.getExtension('WEBGL_debug_renderer_info');return gl.getParameter(d.UNMASKED_RENDERER_WEBGL)})()`));

async function frame(t, i) {
  await browser.evaluate(`window.__seek(${t}, ${i})`);
  const { data } = await browser.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false, fromSurface: true });
  return Buffer.from(data, 'base64');
}

if (args.stills) {
  const dir = resolve(OUT, 'stills'); mkdirSync(dir, { recursive: true });
  for (const t of String(args.stills).split(',').map(Number)) {
    writeFileSync(resolve(dir, `t-${t.toFixed(2).padStart(6, '0')}.png`), await frame(t, Math.round(t * 60)));
    console.log('still', t);
  }
} else {
  const [a, b] = args.range ? String(args.range).split(',').map(Number) : [0, DUR];
  const n = Math.round((b - a) * FPS);
  const video = resolve(OUT, 'video.mp4');
  const ff = spawn('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'image2pipe', '-framerate', String(FPS), '-i', '-',
    '-c:v', 'libx264', '-preset', 'slow', '-crf', '16', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', video], { stdio: ['pipe', 'inherit', 'inherit'] });
  const t0 = Date.now();
  for (let i = 0; i < n; i++) {
    const png = await frame(a + i / FPS, i);
    if (!ff.stdin.write(png)) await new Promise((r) => ff.stdin.once('drain', r));
    if (i % 120 === 0) console.log(`frame ${i}/${n}  ${((Date.now() - t0) / 1000).toFixed(1)}s`);
  }
  ff.stdin.end();
  await new Promise((r, j) => ff.on('exit', (c) => (c === 0 ? r() : j(new Error('ffmpeg ' + c)))));
  console.log(`encoded ${n} frames in ${((Date.now() - t0) / 1000).toFixed(1)}s → ${video}`);
  const wav = resolve('../../../scratch/promo-audio/promo.wav');
  if (existsSync(wav) && !args.range) {
    const final = resolve(OUT, 'spacetime-promo-studio.mp4');
    await new Promise((r, j) => spawn('ffmpeg', ['-y', '-loglevel', 'error', '-i', video, '-i', wav, '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k', '-shortest', final], { stdio: 'inherit' }).on('exit', (c) => (c === 0 ? r() : j(new Error('mux ' + c)))));
    console.log('muxed →', final);
  }
}
browser.close(); srv.close();
