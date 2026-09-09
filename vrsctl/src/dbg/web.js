'use strict';
(() => {
const $ = selector => document.querySelector(selector);
const form = $('#filters'), query = $('#query'), container = $('#recording'), scroller = $('#calls');
const panels = $('#inspectors');
const expanded = new Set(), collapsed = new Set(), pinned = [], runNumbers = new Map();
let snapshot = null, records = new Map(), selected = null, nextRun = 0, allInitially = false;
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
const shellQuote = value => `'${value.replaceAll("'", "'\\''")}'`;
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
  const terms = queryTokens(query.value).filter(term => !term.startsWith(`${key}::`));
  terms.push(`${key}::${quote(value)}`);
  query.value = terms.join(' '); force = true; refresh();
}
function command() {
  $('#cli').textContent = `vrsctl dbg${appliedQuery.trim() ? ` --filter ${shellQuote(appliedQuery)}` : ''}${allInitially || expanded.size || $('#group').value === 'source' || $('#order').value === 'slowest' ? ' --all' : ''}`;
}
function choose(id, compare = false) {
  if (compare) {
    if (pinned.includes(id)) pinned.splice(pinned.indexOf(id), 1);
    else if (pinned.length < 3) pinned.push(id);
  } else selected = id;
  render();
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
  const wrap = el('div', undefined, `row ${r.status}${selected === r.id || pinned.includes(r.id) ? ' selected' : ''}${fresh.has(r.id) ? ' fresh' : ''}`);
  wrap.dataset.callId = r.id;
  const line = el('div', undefined, 'line'); wrap.append(line);
  const arrow = button(!flat && nested.length ? (opened ? '▾' : '▸') : '·', 'arrow', () => {
    if (opened) collapsed.add(r.id); else { collapsed.delete(r.id); expanded.add(r.id); }
    render();
  });
  arrow.disabled = flat || !nested.length;
  arrow.setAttribute('aria-label', flat || !nested.length ? 'No collapsed children in this view' : `${opened ? 'Collapse' : 'Expand'} calls inside ${r.site.form}`);
  arrow.setAttribute('aria-expanded', String(!flat && opened));
  arrow.dataset.focusKey = `expand:${r.id}`;
  const sourceCell = el('div', undefined, 'source-cell');
  if (r.kind === 'callback') sourceCell.append(el('span', 'invoke', 'invocation'));
  if (r.site.generated && r.kind !== 'callback') sourceCell.append(el('span', 'generated', 'invocation'));
  const source = button(r.site.form, 'source', event => choose(r.id, event.shiftKey));
  source.title = 'Inspect this invocation; Shift-click to pin a comparison';
  source.dataset.focusKey = `select:${r.id}`;
  sourceCell.append(source);
  const loc = button(shortLocation(r), 'link location', () => addFilter('file', location(r)));
  loc.title = `Filter history at ${location(r)}`; sourceCell.append(loc);
  const result = el('span', preview(r), 'result'); result.title = preview(r);
  const time = el('span', undefined, 'duration'); time.dataset.duration = r.id;
  line.append(arrow, sourceCell, result, time);
  if (!flat && opened && nested.length) {
    const inner = el('div', undefined, 'children');
    for (const child of nested) inner.append(row(child, children, fresh, next));
    wrap.append(inner);
  }
  return wrap;
}
function inspector(r, isPinned) {
  const panel = el('section', undefined, 'inspector'); panel.dataset.inspector = r.id;
  const head = el('div', undefined, 'inspector-head');
  head.append(el('strong', isPinned ? 'Pinned comparison' : 'Selected call'));
  const pin = button(isPinned ? 'Unpin' : 'Pin comparison', '', () => choose(r.id, true));
  pin.disabled = !isPinned && pinned.length >= 3;
  pin.title = pin.disabled ? 'Up to three pinned comparisons' : 'Keep this call beside other inspectors';
  head.append(pin, button('×', '', () => {
    const i = pinned.indexOf(r.id); if (i >= 0) pinned.splice(i, 1);
    if (selected === r.id) selected = null;
    render();
  }));
  head.lastChild.setAttribute('aria-label', 'Close inspector'); panel.append(head);
  panel.append(el('pre', r.site.expression || r.site.form));
  if (r.kind === 'callback') panel.append(el('p', 'This function was invoked by its parent call. The result below is what this invocation returned.', 'call-note'));
  const links = el('div', undefined, 'links');
  links.append(button(r.site.file, 'link', () => addFilter('file', r.site.file)), button(`line ${r.site.line}:${r.site.column}`, 'link', () => addFilter('file', location(r))));
  panel.append(links);
  panel.append(el('h2', 'Inputs'));
  if (!r.arguments.length) panel.append(el('p', r.kind === 'scope' ? 'Evaluation scope' : 'No arguments', 'muted'));
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
  panel.append(el('p', 'Elapsed time includes waiting and child calls.', 'call-note'));
  panel.append(el('h2', 'Context'));
  const identity = el('div', undefined, 'links');
  identity.append(button(`Evaluation ${runNumbers.get(r.run) || ''}`, 'link', () => addFilter('run', r.run)), button('This call and children', 'link', () => addFilter('call', r.id)));
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
  command(); if (!snapshot) return;
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
        heading.append(button(key, 'link', () => addFilter('file', key)), el('span', `${items.length} invocations`, 'muted'));
        items.sort((a,b) => direction * (a.started_ms - b.started_ms || a.sequence - b.sequence));
      } else {
        const scope = records.get(key);
        heading.append(button(`Evaluation ${runNumbers.get(key)}`, 'link', () => addFilter('run', key)), el('span', new Date(items[0].started_ms).toLocaleTimeString(), 'muted'));
        if (scope) {
          heading.append(button(shortLocation(scope), 'link', () => addFilter('file', location(scope))));
          const state = button(preview(scope), `link${scope.status === 'running' ? ' running-label' : ''}`, () => choose(scope.id));
          state.title = 'Inspect the whole evaluation'; heading.append(state);
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
  if (!container.childElementCount) container.append(el('div', 'No matching calls yet. Evaluate a (dbg! …) block.', 'empty'));
  panels.replaceChildren();
  const inspectorIds = [...pinned];
  if (selected && !inspectorIds.includes(selected)) inspectorIds.push(selected);
  panels.classList.toggle('comparing', inspectorIds.length > 1);
  for (const id of inspectorIds) {
    const r = records.get(id); if (!r) continue;
    const panel = inspector(r, pinned.includes(id)); panels.append(panel); panel.scrollTop = panelScroll.get(id) || 0;
  }
  if (!panels.childElementCount) panels.append(el('div', 'Select a call to inspect its inputs and result. Pin calls to compare them side by side.', 'inspector-empty'));
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
    if (!response.ok) throw Error(await response.text());
    const next = await response.json();
    if (query.value !== requested) return;
    $('#query-error').textContent = ''; $('#connection').textContent = '● Live';
    const nextKey = JSON.stringify([next.cursor, next.dropped, next.evicted, requested, next.matches.slice().sort()]);
    if (!force && nextKey === cursor) return;
    const queryChanged = appliedQuery !== requested;
    const wasAtEdge = liveEdge();
    const fresh = new Set(), oldRecords = records;
    records = new Map(next.records.map(r => [r.id, r]));
    for (const r of next.records) {
      if (!runNumbers.has(r.run)) runNumbers.set(r.run, ++nextRun);
      if (snapshot && !queryChanged && oldRecords.get(r.id)?.sequence !== r.sequence) fresh.add(r.id);
    }
    const added = snapshot && !queryChanged ? next.records.filter(r => r.kind !== 'scope' && !oldRecords.has(r.id) && next.matches.includes(r.id)).length : 0;
    if (!wasAtEdge && added) { newCount += added; $('#new').textContent = `${newCount} new ${newCount === 1 ? 'call' : 'calls'}`; $('#new').hidden = false; }
    if (queryChanged) { newCount = 0; $('#new').hidden = true; }
    snapshot = next; appliedQuery = requested; cursor = nextKey; force = false;
    for (const set of [expanded, collapsed]) for (const id of set) if (!records.has(id)) set.delete(id);
    for (let i = pinned.length - 1; i >= 0; i--) if (!records.has(pinned[i])) pinned.splice(i, 1);
    if (selected && !records.has(selected)) selected = null;
    const runs = new Set(next.records.map(r => r.run));
    for (const run of runNumbers.keys()) if (!runs.has(run)) runNumbers.delete(run);
    $('#notice').textContent = next.dropped || next.evicted ? `${next.dropped} observations dropped · ${next.evicted} records evicted. Older calls may no longer be available.` : '';
    render(fresh, !queryChanged && wasAtEdge && added > 0);
  } catch (error) {
    $('#query-error').textContent = `${error.message}. Showing the previous results.`;
    $('#connection').textContent = 'View not updated';
  } finally { busy = false; }
}
form.addEventListener('submit', event => { event.preventDefault(); force = true; refresh(); });
form.addEventListener('input', () => { clearTimeout(filterTimer); filterTimer = setTimeout(() => { force = true; refresh(); }, 180); });
form.addEventListener('reset', () => setTimeout(() => { force = true; refresh(); }, 0));
$('#group').addEventListener('change', () => { render(); jumpToLive(); });
$('#order').addEventListener('change', () => { render(); jumpToLive(); });
$('#new').addEventListener('click', jumpToLive);
scroller.addEventListener('scroll', () => { if (liveEdge()) { newCount = 0; $('#new').hidden = true; } });
(async () => {
  try {
    const config = await (await fetch('config')).json(); query.value = config.filter; allInitially = config.all;
  } catch (error) { $('#query-error').textContent = error.message; }
  await refresh(); setInterval(refresh, 350); setInterval(updateTimers, 100);
})();

})();
