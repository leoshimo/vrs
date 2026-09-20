import {inkEffect} from '../assets/visuals/ink.js';

const dark = matchMedia('(prefers-color-scheme: dark)');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
const well = document.querySelector('.orb-well');
const canvas = document.querySelector('#live-orb');
const ink = inkEffect(document.querySelector('#hero-mark'));
const exporting = new URLSearchParams(location.search).has('export');
if (exporting) {
  document.body.dataset.export = 'true';
  window.renderInkAt = (seconds, duration) => ink.renderAt(seconds, duration);
}
let orbLoading;
const paused = () => reduced.matches;

// Keep the fallback available if WebGL is unavailable or the context is lost.
new MutationObserver(() => {
  well.dataset.ready = String(canvas.dataset.failed === 'false');
}).observe(canvas, {attributes: true, attributeFilter: ['data-failed']});

async function syncMotion() {
  ink.setPaused(paused());
  if (!paused() && !orbLoading) {
    orbLoading = import('../assets/visuals/sphere.js').catch(() => { well.dataset.ready = 'false'; });
  }
  if (orbLoading) {
    await orbLoading;
    window.setOrbView?.({dark: dark.matches, paused: paused()});
  }
  await ink.ready;
  document.body.dataset.visualsReady = 'true';
}
reduced.addEventListener('change', syncMotion);
dark.addEventListener('change', syncMotion);
syncMotion();

const tagline = document.querySelector('.landing-copy p:first-child');
const hero = document.querySelector('.hero');
function fitTagline() {
  tagline.style.removeProperty('--fitted-tagline');
  const range = document.createRange(); range.selectNodeContents(tagline);
  const width = range.getBoundingClientRect().width;
  const available = hero.clientWidth - 4;
  if (width > available) {
    tagline.style.setProperty('--fitted-tagline', (parseFloat(getComputedStyle(tagline).fontSize) * available / width) + 'px');
  }
}
let fittedWidth;
new ResizeObserver(([entry]) => {
  if (entry.contentRect.width === fittedWidth) return;
  fittedWidth = entry.contentRect.width;
  // Font sizing changes the hero's height; defer writes and react only to width.
  requestAnimationFrame(fitTagline);
}).observe(hero);
document.fonts.ready.then(fitTagline);
