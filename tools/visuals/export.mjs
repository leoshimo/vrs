import {chromium} from 'playwright';
import sharp from 'sharp';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {serve} from './server.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const assets = path.join(root, 'assets/visuals');
const run = (command, args) => {
  const result = spawnSync(command, args, {cwd: root, encoding: 'utf8'});
  if (result.error || result.status !== 0) throw Error(result.error?.message || result.stderr || result.stdout);
  return result.stdout;
};
// Fixed export recipe. Edit here, then run export-visuals; no command flags.
const size = {width: 880, height: 232}, frameCount = 80, fps = 10;
run('ffmpeg', ['-version']);
for (const [name, box] of [['n16', {left:119, top:274, width:1303, height:426}],
                         ['manicule', {left:161, top:191, width:1395, height:612}]]) {
  let image = sharp(path.join(assets, 'source', name + '.png')).extract(box);
  image = image.resize({width: name === 'manicule' ? 144 : 870});
  await image.png({palette:true, colours:256, dither:0}).toFile(path.join(assets, (name === 'n16' ? 'logomark' : name) + '.png'));
}
const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'vrs-visuals-'));
let browser, server;
try {
  const site = path.join(temporary, 'site');
  run('uv', ['run', '--locked', 'tools/docs/build.py', '--output', site]);
  server = await serve(site, '/vrs/');
  browser = await chromium.launch();
  for (const theme of ['light', 'dark']) {
    const page = await browser.newPage({viewport: size, deviceScaleFactor: 1, colorScheme: theme, reducedMotion: 'no-preference'});
    await page.clock.install({time: new Date('2026-09-18T12:00:00Z')});
    await page.goto(server.url + 'docs/index.html?export=1');
    await page.waitForFunction(() => document.body.dataset.visualsReady === 'true' && window.orbReady);
    await page.evaluate(() => document.fonts.ready);
    await page.clock.pauseAt(new Date('2026-09-18T12:00:02Z'));
    await page.screenshot({path: path.join(assets, `readme-${theme}.png`)});
    const sphere = await page.locator('#live-orb').evaluate(canvas => canvas.toDataURL('image/png').split(',')[1]);
    await fs.writeFile(path.join(assets, `sphere-${theme}.png`), Buffer.from(sphere, 'base64'));
    const frames = path.join(temporary, theme); await fs.mkdir(frames);
    for (let i = 0; i < frameCount; i++) {
      await page.screenshot({path: path.join(frames, String(i).padStart(3, '0') + '.png')});
      await page.clock.runFor(1000 / fps);
    }
    run('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-framerate', String(fps), '-i', path.join(frames, '%03d.png'),
      '-vf', 'split[s0][s1];[s0]palettegen=max_colors=32:stats_mode=diff[p];[s1][p]paletteuse=dither=none',
      '-loop', '0', path.join(assets, `readme-${theme}.gif`)]);
    console.log(`Exported ${theme}: PNG, GIF, sphere fallback`);
    await page.close();
  }
} finally {
  await browser?.close(); await server?.close();
  await fs.rm(temporary, {recursive:true, force:true});
}
