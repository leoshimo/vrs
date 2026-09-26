import {chromium, webkit} from 'playwright';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import sharp from 'sharp';
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
const vennPicture = [...readme.matchAll(/<picture>[\s\S]*?<\/picture>/g)][1][0];
const originalVenn = await sharp(path.join(root,'assets/visuals/source/venn.png')).ensureAlpha().raw().toBuffer();
for(const dark of [false,true]) {
  const file = `assets/vrs-venn${dark?'-dark':''}.png`;
  const {data,info} = await sharp(path.join(root,file)).ensureAlpha().raw().toBuffer({resolveWithObject:true});
  assert.equal(data.length,originalVenn.length);
  assert.equal(data[3],0,`${file}: exterior paper remains`);
  // Sample clear paper inside each lobe and the central intersection.
  for(const [x,y] of [[625,215],[145,460],[1100,480],[300,1000],[925,1000],[625,800]]) {
    assert.equal(data[(y*info.width+x)*4+3],255,`${file}: circle fill erased`);
  }
  for(let i=0;i<data.length;i+=4) if(data[i+3]) {
    for(let c=0;c<3;c++) assert.equal(data[i+c],dark?255-originalVenn[i+c]:originalVenn[i+c],`${file}: artwork changed`);
  }
  await fs.copyFile(path.join(root,file),path.join(site,file));
}
for(const theme of ['light','dark']) for(const ext of ['gif','png']) {
  const file = `assets/visuals/readme-${theme}.${ext}`;
  const image = sharp(path.join(root,file),{animated:true});
  const {data,info} = await image.ensureAlpha().raw().toBuffer({resolveWithObject:true});
  const frameHeight = 232, frameBytes = info.width * frameHeight * 4;
  for(let start=0;start<data.length;start+=frameBytes) {
    for(const pixel of [0,info.width-1,info.width*(frameHeight-1),info.width*frameHeight-1]) {
      assert.equal(data[start+pixel*4+3],0,`${file}: opaque background in frame ${start/frameBytes}`);
    }
    let visible=0;
    for(let i=start+3;i<start+frameBytes;i+=4) if(data[i]>0) visible++;
    assert(visible>1000,`${file}: missing artwork in frame ${start/frameBytes}`);
  }
  if(ext==='gif') {
    const metadata = await image.metadata();
    assert.equal(metadata.loop,0,`${file}: does not repeat indefinitely`);
    assert(metadata.pages>1,`${file}: animation missing`);
    // Measure visible changes after decoding the final GIF. Transparent RGB
    // does not contribute, and the loop join should behave like any other step.
    const difference = (a,b) => {
      let total=0;
      for(let i=0;i<frameBytes;i+=4) {
        const ai=a*frameBytes+i, bi=b*frameBytes+i;
        const aa=data[ai+3], ba=data[bi+3];
        total+=Math.abs(aa-ba);
        for(let c=0;c<3;c++) total+=Math.abs(data[ai+c]*aa/255-data[bi+c]*ba/255);
      }
      return total/frameBytes;
    };
    const steps=Array.from({length:metadata.pages-1},(_,i)=>difference(i,i+1)).sort((a,b)=>a-b);
    assert(steps[Math.floor(steps.length/2)]>.01,`${file}: animation is frozen`);
    const seam=difference(metadata.pages-1,0), typical=steps[Math.floor(steps.length*.95)];
    assert(seam<=typical*1.5,`${file}: visible loop jump (${seam} vs typical ${typical})`);
  }
  await fs.copyFile(path.join(root,file),path.join(site,file));
}
await fs.writeFile(path.join(site,'readme-check.html'),
  '<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">'+
  '<style>body{margin:0;padding:32px;background:#fff}img{max-width:100%}@media(prefers-color-scheme:dark){body{background:#0d1117}}</style><div id="readme-header">'+picture+'</div><div id="readme-venn">'+vennPicture+'</div>');
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
      for (const document of ['index','tour','design','manual']) {
        for (const width of [1280,960,760,480,375,320]) {
          await page.setViewportSize({width,height:900});
          await page.goto(document==='index' ? server.url : server.url+'docs/'+document+'.html');
          if (document === 'index') assert.equal(page.url(),server.url,'Homepage navigated away from the root');
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
    await page.locator('.wordmark').click();
    assert.equal(new URL(page.url()).pathname,new URL(server.url+'index.html').pathname);
    // Existing links to the old landing URL still render the same page.
    await page.goto(server.url+'docs/index.html');
    await page.waitForFunction(()=>document.body.dataset.visualsReady==='true');
    assert.equal(await page.locator('.tour-link').getAttribute('href'),'tour.html');
    assert.equal(await page.locator('link[rel="canonical"]').getAttribute('href'),'https://vrs.computer/');
    await page.goto(server.url);
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
    await staticPage.goto(server.url);
    assert.equal(staticPage.url(),server.url,'No-JS homepage navigated away from the root');
    assert(await staticPage.locator('.orb-fallback').isVisible());
    assert(await staticPage.locator('.logomark').isVisible());
    assert(await staticPage.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'No-JS overflow');
    await staticPage.locator('.tour-link').click(); assert(staticPage.url().endsWith('/docs/tour.html'));
    // WebGL unavailable: keep the PNG fallback while the logo still animates.
    const fallback=await browser.newPage({viewport:{width:1280,height:900},reducedMotion:'no-preference'});
    await fallback.addInitScript(()=>{const original=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){return type.startsWith('webgl')?null:original.call(this,type,...args);};});
    await fallback.goto(server.url); await fallback.waitForFunction(()=>document.body.dataset.visualsReady==='true');
    assert(await fallback.locator('.orb-fallback').isVisible());
    assert.equal(await fallback.locator('.orb-well').getAttribute('data-ready'),'false');
    for(const width of [320,375,1280]) for(const colorScheme of ['light','dark']) for(const reducedMotion of ['reduce','no-preference']) {
      await page.setViewportSize({width,height:900});
      await page.emulateMedia({colorScheme,reducedMotion});
      await page.goto(server.url+'readme-check.html');
      const expected=`readme-${colorScheme}.${reducedMotion==='reduce'?'png':'gif'}`;
      await page.waitForFunction(expected=>{const i=document.querySelector('picture img');return i.complete&&i.naturalWidth>0&&i.currentSrc.endsWith(expected);},expected);
      const ratio=await page.locator('#readme-header img').evaluate(i=>{const r=i.getBoundingClientRect();return{displayed:r.width/r.height,intrinsic:i.naturalWidth/i.naturalHeight};});
      assert(Math.abs(ratio.displayed-ratio.intrinsic)<.005,`${name} README distorted at ${width}px`);
      const vennExpected=`vrs-venn${colorScheme==='dark'?'-dark':''}.png`;
      await page.waitForFunction(expected=>{const i=document.querySelector('#readme-venn img');return i.complete&&i.naturalWidth>0&&i.currentSrc.endsWith(expected);},vennExpected);
      const vennRatio=await page.locator('#readme-venn img').evaluate(i=>{const r=i.getBoundingClientRect();return r.width/r.height;});
      assert(Math.abs(vennRatio-1)<.005,`${name} Venn distorted at ${width}px`);
      if(name==='chromium' && width===1280) await page.screenshot({path:path.join(artifacts,`readme-${colorScheme}-${reducedMotion}.png`)});
    }
    console.log(`${name}: 48 page layouts, navigation, reduced motion, no-JS, WebGL fallback, and README proportions passed`);
    await browser.close(); browser=undefined;
  }
} catch(error) {
  failures.push(String(error)); throw error;
} finally {
  await fs.writeFile(path.join(artifacts,'report.json'),JSON.stringify({reports,failures},null,2)+'\n');
  await browser?.close(); await server.close(); await fs.rm(temporary,{recursive:true,force:true});
}
