// Reading controls shared by the tour, essays, and manual.
const toc = document.querySelector('#table-of-contents');

if (toc) {
  document.documentElement.classList.add('enhanced');
  const compact = matchMedia('(max-width:959px)');
  const links = [...toc.querySelectorAll('a[href^="#"]')];
  const linkedIds = new Set(links.map(a => a.getAttribute('href').slice(1)));
  const headings = [...document.querySelectorAll('.outline-2 h2, .outline-3 h3, .outline-4 h4')]
    .filter(h => linkedIds.has(h.id));

  function syncControl() {
    toc.open = !compact.matches;
    toc.querySelector('summary').tabIndex = compact.matches ? 0 : -1;
  }

  function update() {
    const current = headings.filter(h => h.getBoundingClientRect().top < 150).at(-1);
    links.forEach(a => { a.classList.remove('current'); a.removeAttribute('aria-current'); });
    toc.querySelectorAll('li.active').forEach(li => li.classList.remove('active'));
    const link = current && links.find(a => a.getAttribute('href') === '#' + current.id);
    if (link) {
      link.classList.add('current');
      link.setAttribute('aria-current', 'location');
      for (let li = link.closest('li'); li; li = li.parentElement.closest('li')) {
        li.classList.add('active');
      }
    }
  }

  toc.addEventListener('click', e => {
    if (e.target.closest('summary') && !compact.matches) e.preventDefault();
    if (e.target.closest('a[href^="#"]') && compact.matches) toc.open = false;
  });
  let queued = false;
  addEventListener('scroll', () => {
    if (!queued) {
      queued = true;
      requestAnimationFrame(() => { queued = false; update(); });
    }
  }, {passive: true});
  compact.addEventListener('change', () => { syncControl(); update(); });
  addEventListener('resize', update);
  syncControl();
  update();
}

document.querySelectorAll('.copy-code').forEach(button => {
  button.addEventListener('click', async () => {
    const value = button.closest('.code-block').querySelector('pre').textContent;
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(value);
      } else {
        const area = document.createElement('textarea');
        area.value = value;
        area.style.position = 'fixed';
        area.style.opacity = '0';
        document.body.append(area);
        area.select();
        const copied = document.execCommand('copy');
        area.remove();
        if (!copied) throw Error('copy unavailable');
      }
      button.textContent = 'Copied';
    } catch {
      button.textContent = 'Select text';
    }
    setTimeout(() => { button.textContent = 'Copy'; }, 1600);
  });
});
