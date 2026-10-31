import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { createHost } from './host.mjs';

const assets = new Map([
  ['/', ['index.html', 'text/html']],
  ['/app.mjs', ['app.mjs', 'text/javascript']],
  ['/style.css', ['style.css', 'text/css']],
]);
const csp = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";

export function createPreview(host = createHost()) {
  const server = createServer(async (req, res) => {
    const address = server.address();
    const authority = `127.0.0.1:${address.port}`;
    const origin = `http://${authority}`;
    const headers = {
      'Content-Security-Policy': csp,
      'Permissions-Policy': 'camera=(), microphone=(), display-capture=()',
      'X-Content-Type-Options': 'nosniff', 'Cache-Control': 'no-store',
    };
    const send = (status, body, type = 'application/json') => {
      res.writeHead(status, { ...headers, 'Content-Type': type });
      res.end(type === 'application/json' ? JSON.stringify(body) : body);
    };
    // This is a loopback diagnostic adapter, NOT a production native endpoint.
    if (req.headers.host !== authority ||
        (req.headers.origin && req.headers.origin !== origin) ||
        req.headers['sec-fetch-site'] === 'cross-site') {
      send(403, { error: 'origin_refused' }); return;
    }
    try {
      if (req.method === 'GET' && assets.has(req.url)) {
        const [name, type] = assets.get(req.url);
        send(200, await readFile(new URL(`./dist/${name}`, import.meta.url)), type);
      } else if (req.method === 'GET' && req.url === '/snapshot') {
        send(200, host.snapshot());
      } else if (req.method === 'POST' && req.url === '/command') {
        if (req.headers.origin !== origin || req.headers['content-type'] !== 'application/json') {
          send(403, { error: 'origin_or_type_refused' }); return;
        }
        let body = '';
        for await (const chunk of req) {
          body += chunk;
          if (Buffer.byteLength(body) > 2048) {
            send(413, { error: 'too_large' }); return;
          }
        }
        const reply = host.command(JSON.parse(body));
        send(reply.ok ? 200 : 409, reply);
      } else send(404, { error: 'not_found' });
    } catch {
      send(400, { error: 'invalid_request' });
    }
  });
  server.requestTimeout = 5000;
  server.headersTimeout = 5000;
  server.maxConnections = 16;
  return server;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const server = createPreview();
  server.listen(Number(process.env.PORT ?? 18841), '127.0.0.1', () => {
    console.log(`Diagnostic browser adapter: http://127.0.0.1:${server.address().port}`);
  });
  for (const signal of ['SIGINT', 'SIGTERM']) {
    process.once(signal, () => { server.closeAllConnections(); server.close(); });
  }
}
