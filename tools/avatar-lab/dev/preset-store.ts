import { readFile, writeFile, rename } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import type { Plugin, Connect } from 'vite';
import { validateSetup } from '../lib/avatar/setup.ts';
const file = fileURLToPath(new URL('../presets/vrs.json', import.meta.url));
const etag = (raw: string) => createHash('sha256').update(raw).digest('hex');
export function presetStore(): Plugin {
  let writeQueue = Promise.resolve();
  const handler: Connect.NextHandleFunction = async (req, res, next) => {
    if (req.url?.split('?')[0] !== '/__avatar/setup') return next();
    res.setHeader('Content-Type', 'application/json');
    res.setHeader('Cache-Control', 'no-store');
    const reply = (code: number, data: unknown) => {
      res.statusCode = code;
      res.end(JSON.stringify(data));
    };
    if (req.method === 'GET') {
      try {
        const raw = await readFile(file, 'utf8');
        reply(200, { setup: JSON.parse(raw), revision: etag(raw) });
      } catch {
        reply(500, { error: 'Could not read presets/vrs.json' });
      }
      return;
    }
    if (req.method !== 'PUT')
      return reply(405, { error: 'Method not allowed' });
    // Only this local app can write this fixed file; no arbitrary paths or cross-origin writes.
    if (
      req.headers.origin !== `http://${req.headers.host}` ||
      !req.headers['content-type']?.startsWith('application/json')
    )
      return reply(403, { error: 'Same-origin JSON required' });
    let body = '';
    try {
      for await (const chunk of req) {
        body += chunk;
        if (Buffer.byteLength(body) > 300000)
          return reply(413, { error: 'Setup too large' });
      }
      const data = JSON.parse(body);
      validateSetup(data.setup);
      writeQueue = writeQueue.then(async () => {
        try {
          const old = await readFile(file, 'utf8');
          if (data.revision !== etag(old))
            return reply(409, {
              error: 'The file changed on disk. Reload it before saving.',
            });
          const raw =
            JSON.stringify({ ...data.setup, initialized: true }, null, 2) +
            '\n';
          const temp = `${file}.tmp`;
          await writeFile(temp, raw, 'utf8');
          await rename(temp, file);
          reply(200, { revision: etag(raw) });
        } catch {
          reply(500, { error: 'Could not save presets/vrs.json' });
        }
      });
      await writeQueue;
    } catch (error) {
      reply(400, {
        error: error instanceof Error ? error.message : 'Invalid setup',
      });
    }
  };
  return {
    name: 'avatar-preset-store',
    configureServer(server) {
      server.middlewares.use(handler);
    },
    configurePreviewServer(server) {
      server.middlewares.use(handler);
    },
    handleHotUpdate(ctx) {
      if (ctx.file === file) return [];
    },
  };
}
