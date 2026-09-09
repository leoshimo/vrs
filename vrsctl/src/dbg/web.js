'use strict';
(() => {
const $ = selector => document.querySelector(selector);
const form = $('#filters'), query = $('#query'), container = $('#recording'), scroller = $('#calls');
const panels = $('#inspectors');
const expanded = new Set(), collapsed = new Set(), selected = new Set(), runNumbers = new Map();
let snapshot = null, records = new Map(), focused = null, anchorId = null, nextRun = 0, allInitially = false;
let busy = false, force = true, cursor = '', filterTimer, appliedQuery = '', newCount = 0;
const el = (tag, text, cls) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (cls) node.className = cls;
  return node;
};
const button = (text, cls, action) => {
  const node = el('button', text, cls); node.type = 'button';
  node.addEventListener('click', action); return node;
};
const location = r => `${r.site.file}:${r.site.line}:${r.site.column}`;
const shortLocation = r => `${r.site.file.split('/').pop()}:${r.site.line}:${r.site.column}`;
const elapsed = r => r.status === 'running' ? Math.max(0, Date.now() - r.started_ms) * 1000 : r.elapsed_us;
const duration = us => us >= 1000000 ? `${(us / 1000000).toFixed(2)}s` : `${(us / 1000).toFixed(2)}ms`;
const preview = r => r.status === 'running' ? 'running' : r.status === 'cancelled' ? 'cancelled' : r.result?.text ?? r.status;
const quote = value => JSON.stringify(value);

// Keep spelling/quoting intact when replacing one clicked filter term.
function queryTokens(text) {
  const tokens = []; let token = '', quoted = false, escaped = false;
  for (const ch of text) {
    if (ch === '"' && !escaped) quoted = !quoted;
    if (/\s/.test(ch) && !quoted) { if (token) tokens.push(token); token = ''; }
    else token += ch;
    escaped = ch === '\\' && !escaped;
  }
  if (token) tokens.push(token);
  return tokens;
}
function addFilter(key, value) {
  const terms = queryTokens(query.value).filter(term => !term.startsWith(`${key}:`));
  terms.push(`${key}:${quote(value)}`);
  query.value = terms.join(' '); force = true; refresh();
}
const callLabel = r => snapshot?.labels?.[r.id] || (r.kind === 'callback' ? 'fn' : r.site.form);
const visibleIds = () => [...container.querySelectorAll('[data-call-id]')].map(node => node.dataset.callId);
function choose(id, event = {}) {
  const ids = visibleIds(), anchor = ids.indexOf(anchorId), end = ids.indexOf(id);
  if (event.shiftKey && anchor >= 0 && end >= 0) {
    selected.clear();
    ids.slice(Math.min(anchor, end), Math.max(anchor, end) + 1).forEach(id => selected.add(id));
  } else if (event.metaKey || event.ctrlKey) {
    if (selected.has(id)) selected.delete(id); else selected.add(id);
    anchorId = id;
  } else {
    selected.clear(); selected.add(id); anchorId = id;
  }
  focused = id;
  render();
  container.focus({preventScroll:true});
}
function removeSelection(id) {
  selected.delete(id);
  if (focused === id) focused = [...selected].at(-1) || null;
  if (anchorId === id) anchorId = focused;
  render();
  container.focus({preventScroll:true});
}
function liveEdge() {
  if ($('#order').value === 'slowest') return false;
  return $('#order').value === 'oldest'
    ? scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 35
    : scroller.scrollTop < 35;
}
function jumpToLive() {
  scroller.scrollTop = $('#order').value === 'oldest' ? scroller.scrollHeight : 0;
  newCount = 0; $('#new').hidden = true;
}
function updateTimers() {
  for (const node of document.querySelectorAll('[data-duration]')) {
    const r = records.get(node.dataset.duration); if (!r) continue;
    const us = elapsed(r); node.textContent = duration(us);
    node.classList.toggle('slow', us >= 100000);
    node.title = 'Elapsed wall time, including waiting and child calls';
  }
}
function row(r, children, fresh, ancestors = new Set(), flat = false) {
  if (ancestors.has(r.id)) return el('div', 'Incomplete call ancestry', 'muted');
  const next = new Set(ancestors); next.add(r.id);
  const nested = children.get(r.id) || [];
  const opened = !collapsed.has(r.id) && (allInitially || expanded.has(r.id));
  const wrap = el('div', undefined, `row ${r.status}${selected.has(r.id) ? ' selected' : ''}${fresh.has(r.id) ? ' fresh' : ''}`);
  wrap.dataset.callId = r.id; wrap.id = `call-${r.id}`;
  wrap.classList.toggle('current', focused === r.id);
  wrap.setAttribute('role', 'treeitem');
  wrap.setAttribute('aria-selected', String(selected.has(r.id)));
  wrap.setAttribute('aria-label', `${callLabel(r)}: ${preview(r)}`);
  if (!flat && nested.length) wrap.setAttribute('aria-expanded', String(opened));
  const line = el('div', undefined, 'line'); wrap.append(line);
  const arrow = button(!flat && nested.length ? (opened ? '⌄' : '›') : '', 'arrow', () => {
    if (opened) collapsed.add(r.id); else { collapsed.delete(r.id); expanded.add(r.id); }
    render();
  });
  arrow.disabled = flat || !nested.length;
  arrow.setAttribute('aria-label', flat || !nested.length ? 'No collapsed children in this view' : `${opened ? 'Collapse' : 'Expand'} calls inside ${callLabel(r)}`);
  arrow.setAttribute('aria-expanded', String(!flat && opened));
  arrow.dataset.focusKey = `expand:${r.id}`;
  const sourceCell = el('div', undefined, 'source-cell');
  if (r.site.generated && r.kind !== 'callback') sourceCell.append(el('span', 'generated', 'invocation'));
  const source = button(callLabel(r), 'source', event => choose(r.id, event));
  source.tabIndex = -1;
  source.title = 'Inspect call; Shift-click selects a range';
  source.dataset.focusKey = `select:${r.id}`;
  sourceCell.append(source);
  if (r.kind === 'callback' && r.arguments.length) {
    const inputs = el('div', r.arguments.map(arg => arg.text).join(', '), 'input-preview');
    inputs.title = 'Arguments'; sourceCell.append(inputs);
  }
  const loc = button(shortLocation(r), 'link location', () => addFilter('file', location(r)));
  loc.title = `Filter history at ${location(r)}`; sourceCell.append(loc);
  const result = el('span', preview(r), 'result'); result.title = preview(r);
  const time = el('span', undefined, 'duration'); time.dataset.duration = r.id;
  line.append(arrow, sourceCell, result, time);
  line.addEventListener('click', event => {
    if (!event.target.closest('button')) choose(r.id, event);
  });
  if (!flat && opened && nested.length) {
    const inner = el('div', undefined, 'children'); inner.setAttribute('role', 'group');
    for (const child of nested) inner.append(row(child, children, fresh, next));
    wrap.append(inner);
  }
  return wrap;
}
function inspector(r) {
  const panel = el('section', undefined, 'inspector'); panel.dataset.inspector = r.id;
  const head = el('div', undefined, 'inspector-head');
  head.append(el('strong', 'Call'), button('×', 'close-inspector', () => removeSelection(r.id)));
  head.lastChild.setAttribute('aria-label', 'Deselect call'); panel.append(head);
  if (r.kind === 'callback') {
    panel.append(el('pre', callLabel(r)));
    panel.append(el('h2', 'Function definition'));
  }
  panel.append(el('pre', r.site.expression || r.site.form));
  const links = el('div', undefined, 'links');
  links.append(button(r.site.file, 'link', () => addFilter('file', r.site.file)), button(`line ${r.site.line}:${r.site.column}`, 'link', () => addFilter('file', location(r))));
  panel.append(links);
  panel.append(el('h2', 'Inputs'));
  if (!r.arguments.length) panel.append(el('p', r.kind === 'scope' ? 'Debug block' : 'No arguments', 'muted'));
  r.arguments.forEach((arg, i) => {
    panel.append(el('div', `Argument ${i + 1}`, 'muted'));
    panel.append(el('pre', arg.text + (arg.truncated ? ' [truncated]' : '')));
  });
  if (r.arguments_truncated) panel.append(el('p', 'Additional arguments omitted', 'muted'));
  panel.append(el('h2', r.status === 'error' ? 'Error' : 'Result'));
  const result = el('pre', preview(r) + (r.result?.truncated ? ' [truncated]' : ''), r.status === 'running' ? 'running-label' : r.status === 'error' ? 'error-text' : 'result-value');
  panel.append(result);
  const outcome = el('div', undefined, 'outcome');
  outcome.append(button(r.status, 'link', () => addFilter('status', r.status)), document.createTextNode(' · '));
  const time = el('span'); time.dataset.duration = r.id; outcome.append(time); panel.append(outcome);
  panel.append(el('h2', 'Context'));
  const identity = el('div', undefined, 'links');
  identity.append(button(`Run ${runNumbers.get(r.run) || ''}`, 'link', () => addFilter('run', r.run)), button('This call and children', 'link', () => addFilter('call', r.id)));
  panel.append(identity);
  const parent = records.get(r.parent);
  if (parent) {
    panel.append(el('p', 'Called from', 'muted'));
    panel.append(button(parent.site.form, 'source', () => choose(parent.id)));
  }
  panel.append(el('p', r.process, 'muted'));
  return panel;
}
function render(fresh = new Set(), follow = false) {
  if (!snapshot) return;
  const activeKey = document.activeElement?.dataset.focusKey;
  const oldTop = scroller.scrollTop;
  const oldAnchor = [...container.querySelectorAll('[data-call-id]')].find(node => node.getBoundingClientRect().top >= scroller.getBoundingClientRect().top + 28);
  const anchor = oldAnchor ? { id: oldAnchor.dataset.callId, top: oldAnchor.getBoundingClientRect().top } : null;
  const panelScroll = new Map([...panels.querySelectorAll('[data-inspector]')].map(node => [node.dataset.inspector, node.scrollTop]));
  const panelLeft = panels.scrollLeft;
  container.replaceChildren();
  const matching = new Set(snapshot.matches), groupBy = $('#group').value, order = $('#order').value;
  const visible = snapshot.records.filter(r => matching.has(r.id));
  const calls = visible.filter(r => r.kind !== 'scope');
  $('#count').textContent = `${calls.length} calls`;
  $('#group').disabled = order === 'slowest';
  const children = new Map();
  for (const r of calls) { if (!children.has(r.parent)) children.set(r.parent, []); children.get(r.parent).push(r); }
  const flat = snapshot.focused || groupBy === 'source' || order === 'slowest';
  if (order === 'slowest') {
    calls.sort((a, b) => Number(b.status === 'running') - Number(a.status === 'running') || elapsed(b) - elapsed(a) || a.id.localeCompare(b.id));
    for (const r of calls) container.append(row(r, children, fresh, new Set(), true));
  } else {
    const groups = new Map();
    for (const r of visible) {
      if (groupBy === 'source' && r.kind === 'scope') continue;
      const key = groupBy === 'source' ? location(r) : r.run;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(r);
    }
    const direction = order === 'oldest' ? 1 : -1;
    const ordered = [...groups.entries()].sort((a, b) => direction * (Math.min(...a[1].map(r => r.started_ms)) - Math.min(...b[1].map(r => r.started_ms)) || (runNumbers.get(a[1][0].run) - runNumbers.get(b[1][0].run))));
    for (const [key, items] of ordered) {
      const group = el('section', undefined, 'group'), heading = el('div', undefined, 'group-heading');
      if (groupBy === 'source') {
        heading.append(button(key, 'link', () => addFilter('file', key)), el('span', `${items.length} calls`, 'muted'));
        items.sort((a,b) => direction * (a.started_ms - b.started_ms || a.sequence - b.sequence));
      } else {
        const scope = records.get(key);
        heading.append(button(`Run ${runNumbers.get(key)}`, 'link', () => addFilter('run', key)), el('span', new Date(items[0].started_ms).toLocaleTimeString(), 'muted'));
        if (scope) {
          heading.append(button(shortLocation(scope), 'link', () => addFilter('file', location(scope))));
          const state = button(preview(scope), `link${scope.status === 'running' ? ' running-label' : ''}`, () => choose(scope.id));
          state.title = 'Inspect this dbg! block'; heading.append(state);
        }
      }
      group.append(heading);
      const itemIds = new Set(items.filter(r => r.kind !== 'scope').map(r => r.id));
      for (const r of items.filter(r => r.kind !== 'scope')) {
        if (!flat && itemIds.has(r.parent)) continue;
        group.append(row(r, children, fresh, new Set(), flat));
      }
      container.append(group);
    }
  }
  if (!container.childElementCount) container.append(el('div', 'No matching calls yet.', 'empty'));
  panels.replaceChildren();
  for (const id of selected) {
    const r = records.get(id); if (!r) continue;
    const panel = inspector(r); panels.append(panel); panel.scrollTop = panelScroll.get(id) || 0;
  }
  if (!panels.childElementCount) panels.append(el('div', 'Select a call to inspect its inputs and result.', 'inspector-empty'));
  if (focused && document.getElementById(`call-${focused}`)) container.setAttribute('aria-activedescendant', `call-${focused}`);
  else container.removeAttribute('aria-activedescendant');
  panels.scrollLeft = panelLeft;
  updateTimers();
  if (follow) jumpToLive();
  else {
    scroller.scrollTop = oldTop;
    if (anchor) {
      const node = [...container.querySelectorAll('[data-call-id]')].find(node => node.dataset.callId === anchor.id);
      if (node) scroller.scrollTop += node.getBoundingClientRect().top - anchor.top;
    }
  }
  if (activeKey) [...document.querySelectorAll('[data-focus-key]')].find(node => node.dataset.focusKey === activeKey)?.focus({preventScroll:true});
}
async function refresh() {
  if (busy) return; busy = true;
  const requested = query.value;
  try {
    const response = await fetch(`api?${new URLSearchParams({filter:requested})}`, {cache:'no-store'});
    if (response.status === 400) {
      $('#query-error').textContent = await response.text(); $('#connection').hidden = true; return;
    }
    if (!response.ok) throw Error(await response.text());
    const next = await response.json();
    if (query.value !== requested) return;
    $('#query-error').textContent = ''; $('#connection').hidden = true;
    const nextKey = JSON.stringify([next.cursor, next.dropped, next.evicted, requested, next.matches.slice().sort()]);
    if (!force && nextKey === cursor) return;
    const queryChanged = appliedQuery !== requested;
    const wasAtEdge = liveEdge();
    const fresh = new Set(), oldRecords = records;
    records = new Map(next.records.map(r => [r.id, r]));
    for (const r of next.records) {
      if (next.matches.includes(r.id) && !runNumbers.has(r.run)) runNumbers.set(r.run, ++nextRun);
      if (snapshot && !queryChanged && oldRecords.get(r.id)?.sequence !== r.sequence) fresh.add(r.id);
    }
    const added = snapshot && !queryChanged ? next.records.filter(r => r.kind !== 'scope' && !oldRecords.has(r.id) && next.matches.includes(r.id)).length : 0;
    if (!wasAtEdge && added) { newCount += added; $('#new').textContent = `${newCount} new ${newCount === 1 ? 'call' : 'calls'}`; $('#new').hidden = false; }
    if (queryChanged) { newCount = 0; $('#new').hidden = true; }
    snapshot = next; appliedQuery = requested; cursor = nextKey; force = false;
    for (const set of [expanded, collapsed, selected]) for (const id of set) if (!records.has(id)) set.delete(id);
    if (focused && !records.has(focused)) focused = null;
    if (anchorId && !records.has(anchorId)) anchorId = null;
    const runs = new Set(next.records.map(r => r.run));
    for (const run of runNumbers.keys()) if (!runs.has(run)) runNumbers.delete(run);
    $('#notice').textContent = next.dropped ? `${next.dropped} observations could not be recorded.` : '';
    render(fresh, !queryChanged && wasAtEdge && added > 0);
  } catch (error) {
    $('#connection').hidden = false; $('#connection').title = error.message;
  } finally { busy = false; }
}
form.addEventListener('submit', event => { event.preventDefault(); force = true; refresh(); });
form.addEventListener('input', () => { clearTimeout(filterTimer); filterTimer = setTimeout(() => { force = true; refresh(); }, 180); });
form.addEventListener('reset', () => setTimeout(() => { force = true; refresh(); }, 0));
$('#group').addEventListener('change', () => { render(); jumpToLive(); });
$('#order').addEventListener('change', () => { render(); jumpToLive(); });
$('#new').addEventListener('click', jumpToLive);
scroller.addEventListener('scroll', () => { if (liveEdge()) { newCount = 0; $('#new').hidden = true; } });
container.addEventListener('keydown', event => {
  if (!['ArrowUp', 'ArrowDown'].includes(event.key) || event.altKey || event.metaKey || event.ctrlKey) return;
  const ids = visibleIds(); if (!ids.length) return;
  event.preventDefault();
  const current = ids.indexOf(focused);
  const index = current < 0 ? (event.key === 'ArrowUp' ? ids.length - 1 : 0)
    : Math.max(0, Math.min(ids.length - 1, current + (event.key === 'ArrowUp' ? -1 : 1)));
  choose(ids[index], event);
  document.getElementById(`call-${ids[index]}`)?.querySelector('.line').scrollIntoView({block:'nearest'});
});
const divider = $('#divider'), workspace = $('#workspace');
function resizeSplit(value) {
  const percent = Math.max(25, Math.min(75, value));
  workspace.style.setProperty('--list-width', `${percent}%`);
  divider.setAttribute('aria-valuenow', String(Math.round(percent)));
}
divider.addEventListener('pointerdown', event => {
  if (event.button !== 0) return;
  event.preventDefault(); divider.focus(); divider.setPointerCapture(event.pointerId);
  workspace.classList.add('resizing');
});
divider.addEventListener('pointermove', event => {
  if (!divider.hasPointerCapture(event.pointerId)) return;
  const rect = workspace.getBoundingClientRect();
  resizeSplit((event.clientX - rect.left) / rect.width * 100);
});
const endResize = () => workspace.classList.remove('resizing');
divider.addEventListener('lostpointercapture', endResize);
divider.addEventListener('pointerup', event => { divider.releasePointerCapture(event.pointerId); endResize(); });
divider.addEventListener('pointercancel', endResize);
divider.addEventListener('keydown', event => {
  if (!['ArrowLeft','ArrowRight','Home','End'].includes(event.key)) return;
  event.preventDefault();
  const value = Number(divider.getAttribute('aria-valuenow'));
  resizeSplit(event.key === 'Home' ? 25 : event.key === 'End' ? 75 : value + (event.key === 'ArrowLeft' ? -2 : 2));
});
document.addEventListener('keydown', event => { if (event.key === 'Escape') $('#help').open = false; });
document.addEventListener('click', event => { if (!event.target.closest('#help')) $('#help').open = false; });
(async () => {
  try {
    const config = await (await fetch('config')).json(); query.value = config.filter; allInitially = config.all;
  } catch (error) { $('#query-error').textContent = error.message; }
  await refresh(); setInterval(refresh, 350); setInterval(updateTimers, 100);
})();

})();
