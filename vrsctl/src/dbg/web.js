'use strict';
const $=s=>document.querySelector(s), form=$('#filters'), container=$('#recording');
const expanded=new Set(), values=new Set(), collapsed=new Set(), hiddenValues=new Set();
let snapshot=null, paused=false, cursor='', filterTimer, busy=false, force=true;
const el=(tag,text,cls)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;if(cls)e.className=cls;return e;};
const button=(text,cls,action)=>{const e=el('button',text,cls);e.type='button';e.addEventListener('click',action);return e;};
const site=r=>`${r.site.file}:${r.site.line}:${r.site.column}`;
const short=id=>id.slice(0,8);
function filters(){return Object.fromEntries(new FormData(form));}
function focusFilter(key,value){for(const name of Object.keys(filters()))form.elements[name].value='';form.elements[key].value=value;force=true;refresh();}
function command(){const f=filters();let s='vrsctl dbg';for(const [key,value] of Object.entries(f))if(value)s+=` --${key} '${value.replaceAll("'","'\\''")}'`;if($('#all').checked)s+=' --all';if($('#details').checked)s+=' --details';$('#cli').textContent=s;}
function toggle(set,id){set.has(id)?set.delete(id):set.add(id);render();}
function row(r,byParent,ancestors=new Set()){
  if(ancestors.has(r.id))return el('div','Cycle in incomplete recording','muted');
  const next=new Set(ancestors);next.add(r.id);
  const children=byParent.get(r.id)||[], wrap=el('div',undefined,`row ${r.status}`), line=el('div',undefined,'line');wrap.append(line);
  const view=$('#group').value, key=`${view}:${r.id}`;
  const opened=!collapsed.has(key)&&($('#all').checked||expanded.has(key)||(view==='run'&&Object.values(filters()).some(Boolean)));
  const showingValues=!hiddenValues.has(r.id)&&($('#details').checked||values.has(r.id));
  const arrow=button(children.length?(opened?'▾':'▸'):'·','arrow',()=>{if(opened){collapsed.add(key);}else{collapsed.delete(key);expanded.add(key);}render();});arrow.disabled=!children.length;arrow.dataset.focusKey=`children:${r.id}`;arrow.setAttribute('aria-label',`${opened?'Collapse':'Expand'} nested calls for ${r.site.form}`);arrow.setAttribute('aria-expanded',String(opened));line.append(arrow);
  const source=button(r.site.form,'source',()=>{if(showingValues){hiddenValues.add(r.id);}else{hiddenValues.delete(r.id);values.add(r.id);}render();});source.dataset.focusKey=`values:${r.id}`;source.title='Inspect actual arguments and result';source.setAttribute('aria-expanded',String(showingValues));line.append(source);
  const loc=button(`${r.site.file.split('/').pop()}:${r.site.line}:${r.site.column}`,'link location',()=>focusFilter('at',site(r)));loc.title=`History at ${site(r)}`;line.append(loc);
  if(r.kind==='callback'||r.site.generated)line.append(el('span',r.kind==='callback'?'callback':'generated','tag'));
  const result=el('span',r.result?.text||r.status,'result');result.title=r.result?.text||r.status;line.append(result,el('span',r.status==='running'?'pending':`${(r.elapsed_us/1000).toFixed(2)}ms`,'duration'));
  if(showingValues){
    const panel=el('div',undefined,'values'),dl=el('dl');panel.append(dl);
    const field=(name,text)=>dl.append(el('dt',name),el('dd',text));
    field('Source',r.site.expression);field('Location',site(r));field('Status',r.status);
    r.arguments.forEach((arg,i)=>field(`Argument ${i+1}`,arg.text+(arg.truncated?' [truncated]':'')));
    if(r.arguments_truncated)field('Arguments','Additional arguments omitted');
    if(r.result)field(r.status==='error'?'Error':'Result',r.result.text+(r.result.truncated?' [truncated]':''));
    const identity=el('div',undefined,'identity');identity.append(button(`Call ${r.id}`,'link',()=>focusFilter('call',r.id)),button(`Run ${r.run}`,'link',()=>focusFilter('run',r.run)),el('span',r.process,'muted'));
    if(r.parent)identity.append(button(`Parent ${r.parent}`,'link',()=>focusFilter('call',r.parent)));
    panel.append(identity);wrap.append(panel);
  }
  if(opened&&children.length){const inner=el('div',undefined,'children');children.forEach(child=>inner.append(row(child,byParent,next)));wrap.append(inner);}
  return wrap;
}
function render(){
  command();if(!snapshot)return;
  const active=document.activeElement;const focusName=active?.dataset.focusKey;
  container.replaceChildren();
  const records=snapshot.records, ids=new Set(records.map(r=>r.id)), byParent=new Map();
  for(const r of records){if(r.parent){if(!byParent.has(r.parent))byParent.set(r.parent,[]);byParent.get(r.parent).push(r);}}
  $('#count').textContent=`${records.length} records`;
  $('#notice').textContent=(snapshot.dropped||snapshot.evicted)?`${snapshot.dropped} observations dropped · ${snapshot.evicted} records evicted. History may be incomplete; some parents or completions may be missing.`:'';
  if(!records.length){const empty=el('div','No observations match these filters.','empty');empty.append(el('code',"(dbg! (map '(2 3) (fn (x) (+ x x))))"),el('div','Evaluate a block in Emacs, the REPL, or a script.'));container.append(empty);return;}
  const groups=new Map();
  if($('#group').value==='source'){
    for(const r of records){const key=site(r);if(!groups.has(key))groups.set(key,[]);groups.get(key).push(r);}
    for(const [location,items] of groups){const group=el('section',undefined,'group'),heading=el('div',undefined,'group-heading');heading.append(button(location,'link',()=>focusFilter('at',location)),el('span',`${items.length} observations`,'muted'));group.append(heading);for(const r of items)group.append(row(r,byParent));container.append(group);}
  }else{
    for(const r of records){if(!groups.has(r.run))groups.set(r.run,[]);groups.get(r.run).push(r);}
    for(const [id,items] of groups){const group=el('section',undefined,'group'),heading=el('div',undefined,'group-heading');heading.append(button(`Run ${short(id)}`,'link',()=>focusFilter('run',id)),el('span',new Date(items[0].started_ms).toLocaleTimeString(),'muted'),el('span',items[0].process,'muted'));group.append(heading);
      for(const r of items.filter(r=>!r.parent||!ids.has(r.parent))){
        // Scope children are immediately useful; deeper calls start collapsed.
        if(r.kind==='scope')expanded.add(`run:${r.id}`);
        group.append(row(r,byParent));
      }
      container.append(group);
    }
  }
  if(focusName){const target=Array.from(container.querySelectorAll('[data-focus-key]')).find(e=>e.dataset.focusKey===focusName);target?.focus({preventScroll:true});}
  // Bound UI state alongside the bounded daemon recording.
  for(const set of [expanded,collapsed])for(const key of set)if(!ids.has(key.slice(key.indexOf(':')+1)))set.delete(key);
  for(const set of [values,hiddenValues])for(const id of set)if(!ids.has(id))set.delete(id);
}
async function refresh(){
  if(busy||paused)return;busy=true;
  try{const response=await fetch(`api?${new URLSearchParams(filters())}`,{cache:'no-store'});if(!response.ok)throw Error(await response.text());const next=await response.json();const key=JSON.stringify([next.cursor,next.dropped,next.evicted,filters()]);snapshot=next;
    $('#connection').textContent='● Live';if(force||key!==cursor){cursor=key;force=false;render();}
  }catch(error){$('#connection').textContent='Disconnected';$('#notice').textContent=error.message;}finally{busy=false;}
}
form.addEventListener('submit',event=>event.preventDefault());form.addEventListener('input',()=>{clearTimeout(filterTimer);filterTimer=setTimeout(()=>{force=true;refresh();},180);});form.addEventListener('reset',()=>setTimeout(()=>{force=true;refresh();},0));
for(const id of ['all','details','group'])$('#'+id).addEventListener('change',()=>{if(id==='all')collapsed.clear();if(id==='details')hiddenValues.clear();render();});
$('#pause').addEventListener('click',()=>{paused=!paused;$('#pause').textContent=paused?'Resume view':'Pause view';$('#connection').textContent=paused?'Paused view · recording continues':'Connecting…';if(!paused)refresh();});
$('#export').addEventListener('click',()=>{if(!snapshot)return;const blob=new Blob([JSON.stringify(snapshot,null,2)],{type:'application/json'}),url=URL.createObjectURL(blob),a=el('a');a.href=url;a.download='vrs-dbg.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);});
(async()=>{try{const config=await(await fetch('config')).json();for(const [key,value]of Object.entries(config.filter))form.elements[key].value=value;$('#all').checked=config.all;$('#details').checked=config.details;}catch(error){$('#notice').textContent=error.message;}await refresh();setInterval(refresh,350);})();
