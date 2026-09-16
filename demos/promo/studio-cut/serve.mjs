// Tiny static server rooted at the repo root (so /stdlib/3d/vendor/three/… and
// /scratch/promo-studio/… both resolve). Exports start() → { port, close }.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';

const ROOT = process.env.SERVE_ROOT ?? new URL('../../../', import.meta.url).pathname;
const MIME = {
  '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript',
  '.json': 'application/json', '.png': 'image/png', '.jpg': 'image/jpeg',
  '.wav': 'audio/wav', '.css': 'text/css', '.ttf': 'font/ttf', '.woff2': 'font/woff2',
};

export function start() {
  return new Promise((resolve) => {
    const server = createServer(async (req, res) => {
      const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname));
      try {
        const body = await readFile(join(ROOT, path));
        res.writeHead(200, { 'content-type': MIME[extname(path)] ?? 'application/octet-stream', 'cache-control': 'no-store' });
        res.end(body);
      } catch {
        res.writeHead(404); res.end('not found: ' + path);
      }
    });
    server.listen(0, '127.0.0.1', () => resolve({ port: server.address().port, close: () => server.close() }));
  });
}
