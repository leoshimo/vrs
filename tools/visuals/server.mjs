import http from 'node:http';
import fs from 'node:fs/promises';
import path from 'node:path';

export async function serve(directory, prefix = '/') {
  const root = path.resolve(directory);
  const types = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.png':'image/png','.gif':'image/gif','.woff2':'font/woff2'};
  const server = http.createServer(async (req, res) => {
    try {
      const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
      if (!pathname.startsWith(prefix)) throw Error('Unknown prefix');
      const file = path.resolve(root, '.' + '/' + pathname.slice(prefix.length), pathname.endsWith('/') ? 'index.html' : '');
      if (!file.startsWith(root + path.sep)) throw Error('Outside root');
      const data = await fs.readFile(file);
      res.writeHead(200, {'Content-Type': types[path.extname(file)] || 'application/octet-stream'}); res.end(data);
    } catch { res.writeHead(404); res.end('Not found'); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  return {url: `http://127.0.0.1:${server.address().port}${prefix}`, close: () => new Promise(resolve => server.close(resolve))};
}
