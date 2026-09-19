import { readFile, writeFile, rename } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';
import { isAvatarSetup } from '../../../vrsjmp/src/avatar/setup-validation';

const path = fileURLToPath(
  new URL('../../../vrsjmp/src/avatar/assigned-setup.json', import.meta.url),
);
const revision = (text: string) =>
  createHash('sha256').update(text).digest('hex');
export function configurationServer(): Plugin {
  let saving = false;
  return {
    name: 'avatar-configuration',
    hotUpdate({ file }) {
      // Save updates the editor through its response; keep previews and playback mounted.
      if (file === path) return [];
    },
    configureServer(server) {
      server.middlewares.use('/__avatar/configuration', async (req, res) => {
        const reply = (status: number, body: string) => {
          res.statusCode = status;
          res.setHeader('Cache-Control', 'no-store');
          res.end(body);
        };
        if (!['GET', 'PUT'].includes(req.method ?? ''))
          return reply(405, 'Method not allowed.');
        if (
          req.headers.origin &&
          req.headers.origin !== `http://${req.headers.host}`
        )
          return reply(403, 'Save must come from this lab.');
        if (req.method === 'PUT' && saving)
          return reply(409, 'Another save is in progress. Try again.');
        const writing = req.method === 'PUT';
        if (writing) saving = true;
        try {
          let text = await readFile(path, 'utf8');
          if (writing) {
            let body = '';
            for await (const chunk of req) {
              body += chunk;
              if (body.length > 128 * 1024)
                return reply(413, 'Configuration is too large.');
            }
            const value = JSON.parse(body);
            if (!isAvatarSetup(value.setup))
              return reply(400, 'Invalid avatar configuration.');
            if (value.revision !== revision(text))
              return reply(
                409,
                'The configuration changed on disk. Reload before saving.',
              );
            text =
              JSON.stringify(
                {
                  appearance: value.setup.appearance,
                  assignments: value.setup.assignments,
                },
                null,
                2,
              ) + '\n';
            const temporary = `${path}.${randomUUID()}.tmp`;
            await writeFile(temporary, text);
            await rename(temporary, path);
          }
          res.setHeader('Content-Type', 'application/json');
          reply(
            200,
            JSON.stringify({
              setup: JSON.parse(text),
              revision: revision(text),
            }),
          );
        } catch (error) {
          reply(
            error instanceof SyntaxError ? 400 : 500,
            'Could not read or save the avatar configuration.',
          );
        } finally {
          if (writing) saving = false;
        }
      });
    },
  };
}
