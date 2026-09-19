import {chromium, webkit} from 'playwright';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {serve} from './server.mjs';
const root = fileURLToPath(new URL('../../', import.meta.url));
const artifacts = fileURLToPath(new URL('./artifacts/', import.meta.url));
await fs.mkdir(artifacts, {recursive:true});
const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'vrs-site-check-'));
const site = path.join(temporary, 'site');
const result = spawnSync('uv', ['run', '--locked', 'tools/docs/build.py', '--output', site], {cwd:root, encoding:'utf8'});
assert.equal(result.status, 0, result.stderr);
// GitHub constrains image width, but preserves an explicit HTML height.
// Exercise the actual README markup with that sizing behavior.
const readme = await fs.readFile(path.join(root,'README.md'),'utf8');
const picture = readme.match(/<picture>[\s\S]*?<\/picture>/)[0];
for(const theme of ['light','dark']) for(const ext of ['gif','png']) {
  const file = `assets/visuals/readme-${theme}.${ext}`;
  await fs.copyFile(path.join(root,file),path.join(site,file));
}
await fs.writeFile(path.join(site,'readme-check.html'),
  '<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">'+
  '<style>body{margin:0;padding:32px}img{max-width:100%}</style>'+picture);
const server = await serve(site, '/vrs/');
const reports = [], failures = [];
let browser;
try {
  for (const [name, engine] of [['chromium',chromium], ['webkit',webkit]]) {
    browser = await engine.launch();
    const context = await browser.newContext({viewport:{width:1280,height:900}, reducedMotion:'reduce', ...(name==='chromium'?{permissions:['clipboard-read','clipboard-write']}:{})});
    const page = await context.newPage();
    const errors = [], missing = [];
    page.on('pageerror', error => errors.push(String(error)));
    page.on('response', response => { if(response.status() >= 400) missing.push(response.url()); });
    for (const theme of ['light','dark']) {
      await page.emulateMedia({colorScheme:theme});
      for (const document of ['index','tour','design','design-experience','manual']) {
        for (const width of [1280,960,760,480,375,320]) {
          await page.setViewportSize({width,height:900});
          await page.goto(server.url+'docs/'+document+'.html');
          await page.evaluate(() => document.fonts.ready);
          if (document === 'index') await page.waitForFunction(() => document.body.dataset.visualsReady === 'true');
          const layout = await page.evaluate(() => {
            const nav = [...document.querySelectorAll('.site-masthead a')].map(e => {const r=e.getBoundingClientRect();return {x:r.x,right:r.right,y:r.y,bottom:r.bottom};});
            const overlaps = nav.some((a,i)=>nav.slice(i+1).some(b=>a.x<b.right&&a.right>b.x&&a.y<b.bottom&&a.bottom>b.y));
            const title = document.querySelector('h1');
            return {overflow:document.documentElement.scrollWidth>innerWidth,navOverlap:overlaps,h1:document.querySelectorAll('h1').length,font:getComputedStyle(title).fontFamily};
          });
          assert(!layout.overflow, `${name} ${document} ${theme} ${width}: horizontal overflow`);
          assert(!layout.navOverlap, `${name} ${document} ${width}: overlapping navigation`);
          assert.equal(layout.h1,1);
          assert(layout.font.includes('Charter'));
          if (document === 'index') {
            const repl=await page.locator('.repl').boundingBox(), tour=await page.locator('.tour-link').boundingBox();
            assert(repl.x+repl.width<=tour.x, `${name} ${width}: REPL/tour collision`);
            assert.equal(await page.locator('.wordmark').count(),0);
            assert.equal(await page.locator('#motion-switch').count(),0);
            assert(await page.locator('.orb-fallback').isVisible());
          } else {
            assert.equal(await page.locator('.document-sidebar .github-source').getAttribute('href'), `https://github.com/leoshimo/vrs/blob/main/docs/${document}.md`);
            assert(await page.locator('.wordmark').isVisible());
            assert.equal(await page.locator('.document-title a').count(),0);
            assert.equal(await page.locator('.github-source').textContent(),'View source');
          }
          if(name==='chromium' && [1280,375].includes(width)) await page.screenshot({path:path.join(artifacts,`${document}-${theme}-${width}.png`)});
          reports.push({engine:name,document,theme,width,...layout});
        }
      }
    }
    await page.setViewportSize({width:1280,height:900});
    await page.goto(server.url+'docs/tour.html');
    const anchor = page.locator('.contents a').first();
    const hash = await anchor.getAttribute('href'); await anchor.click();
    assert.equal(new URL(page.url()).hash,hash);
    await page.evaluate(()=>scrollTo(0,document.body.scrollHeight));
    await page.waitForFunction(()=>{const links=[...document.querySelectorAll('.contents a')];return links.at(-1).getAttribute('aria-current')==='location';});
    await page.locator('.copy-code').first().click();
    if(name==='chromium') {
      await page.waitForFunction(()=>document.querySelector('.copy-code').textContent==='Copied');
      assert.equal(await page.evaluate(()=>navigator.clipboard.readText()),await page.locator('.code-block pre').first().textContent());
    }
    await page.goto(server.url+'docs/index.html');
    await page.keyboard.press(name==='webkit' && process.platform==='darwin' ? 'Alt+Tab' : 'Tab'); assert.equal(await page.evaluate(()=>document.activeElement.textContent),'Tour');
    // Motion follows the system preference, including changes while open.
    await page.emulateMedia({reducedMotion:'no-preference'});
    await page.waitForFunction(()=>document.querySelector('.orb-well').dataset.ready==='true');
    const motion=page.locator('.ink-motion');
    const before=await motion.getAttribute('data-phase'); await page.waitForTimeout(250);
    assert.notEqual(await motion.getAttribute('data-phase'),before);
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.waitForTimeout(100);
    const stopped=await motion.getAttribute('data-phase'); await page.waitForTimeout(150);
    assert.equal(await motion.getAttribute('data-phase'),stopped);
    await page.emulateMedia({colorScheme:'light',reducedMotion:'no-preference'});
    await page.reload(); await page.waitForFunction(()=>document.querySelector('.orb-well').dataset.ready==='true');
    await page.screenshot({path:path.join(artifacts,`landing-live-${name}.png`)});
    assert.deepEqual(errors,[]); assert.deepEqual(missing,[]);
    // No JS must preserve the composition and useful links at phone widths.
    const staticPage=await browser.newPage({javaScriptEnabled:false,viewport:{width:320,height:800}});
    await staticPage.goto(server.url+'docs/index.html');
    assert(await staticPage.locator('.orb-fallback').isVisible());
    assert(await staticPage.locator('.logomark').isVisible());
    assert(await staticPage.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'No-JS overflow');
    await staticPage.locator('.tour-link').click(); assert(staticPage.url().endsWith('/docs/tour.html'));
    // WebGL unavailable: keep the PNG fallback while the logo still animates.
    const fallback=await browser.newPage({viewport:{width:1280,height:900},reducedMotion:'no-preference'});
    await fallback.addInitScript(()=>{const original=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){return type.startsWith('webgl')?null:original.call(this,type,...args);};});
    await fallback.goto(server.url+'docs/index.html'); await fallback.waitForFunction(()=>document.body.dataset.visualsReady==='true');
    assert(await fallback.locator('.orb-fallback').isVisible());
    assert.equal(await fallback.locator('.orb-well').getAttribute('data-ready'),'false');
    for(const width of [320,375,1280]) for(const colorScheme of ['light','dark']) for(const reducedMotion of ['reduce','no-preference']) {
      await page.setViewportSize({width,height:900});
      await page.emulateMedia({colorScheme,reducedMotion});
      await page.goto(server.url+'readme-check.html');
      const expected=`readme-${colorScheme}.${reducedMotion==='reduce'?'png':'gif'}`;
      await page.waitForFunction(expected=>{const i=document.querySelector('picture img');return i.complete&&i.naturalWidth>0&&i.currentSrc.endsWith(expected);},expected);
      const ratio=await page.locator('picture img').evaluate(i=>{const r=i.getBoundingClientRect();return{displayed:r.width/r.height,intrinsic:i.naturalWidth/i.naturalHeight};});
      assert(Math.abs(ratio.displayed-ratio.intrinsic)<.005,`${name} README distorted at ${width}px`);
    }
    console.log(`${name}: 60 page layouts, navigation, reduced motion, no-JS, WebGL fallback, and README proportions passed`);
    await browser.close(); browser=undefined;
  }
} catch(error) {
  failures.push(String(error)); throw error;
} finally {
  await fs.writeFile(path.join(artifacts,'report.json'),JSON.stringify({reports,failures},null,2)+'\n');
  await browser?.close(); await server.close(); await fs.rm(temporary,{recursive:true,force:true});
}
