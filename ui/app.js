"use strict";
(() => {
  const token = new URLSearchParams(location.hash.slice(1)).get('token') || '';
  history.replaceState(null, '', location.pathname);
  const $ = s => document.querySelector(s);
  const esc = v => String(v ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const themes = [['night','Uika'],['paper','Confluence'],['panel','Panel'],['pc98','Mariyaholic-98'],['oak','Argonaut Oak']];
  const state = {page:'history',query:'',platform:'',channel:'',after:'',mode:'fts',offset:0,info:null,selected:new Set(),profiles:[],seconds:1800,seed:null,collecting:false,theme:'night',density:'comfortable'};
  let epoch=0, opener=null, currentItem=null, confirmed=null;
  const date = t => t == null ? 'Unknown' : new Date(t*1000).toLocaleString();
  const day = t => t == null ? 'No opening recorded' : new Date(t*1000).toLocaleDateString(undefined,{year:'numeric',month:'short',day:'numeric'});
  const time = t => t == null ? '' : new Date(t*1000).toLocaleTimeString(undefined,{hour:'2-digit',minute:'2-digit'});
  const duration = s => s == null ? 'Unknown' : `${Math.floor(s/60)}m ${s%60}s`;
  const browser = source => (source || '').split(':')[0] || 'Unknown';
  function message(text='',error=false){$('#message').textContent=text;$('#message').classList.toggle('error',error);}
  async function api(path,body){
    const response=await fetch(`/api/${path}`,{method:body===undefined?'GET':'POST',headers:{'X-Omoi-Token':token,...(body===undefined?{}:{'Content-Type':'application/json'})},...(body===undefined?{}:{body:JSON.stringify(body)})});
    const data=await response.json();
    if(!response.ok)throw new Error(data.error || 'Request failed.');
    $('#connection-state').textContent='Connected';
    return data;
  }
  function fail(error){message(error.message==='Failed to fetch'?'Server unavailable. Run omoi ui and open its launch URL.':error.message,true);$('#connection-state').textContent='Needs attention';}
  function applyTheme(){document.body.dataset.skin=state.theme;document.body.dataset.face=state.theme==='paper'?'sumimi':'doloris';document.body.dataset.density=state.density;}
  function preferenceKey(){return `omoi:${state.info.catalogue}:appearance`;}
  function savePreferences(){try{localStorage.setItem(preferenceKey(),JSON.stringify({theme:state.theme,density:state.density}));}catch{message('Browser storage is unavailable. Appearance will reset when closed.');}}
  async function info(){state.info=await api('info');$('#nav-count').textContent=state.info.items;}
  async function render(){
    const version=++epoch,page=state.page;
    $('#page-title').textContent={history:'History',suggestions:'Suggestions',sources:'Sources',settings:'Settings'}[page];
    document.querySelectorAll('nav [data-page]').forEach(b=>b.dataset.page===page?b.setAttribute('aria-current','page'):b.removeAttribute('aria-current'));
    $('#content').setAttribute('aria-busy','true');
    $('#content').innerHTML='<p class="fine-print">Loading…</p>';
    try {
      let html;
      if(page==='history')html=await historyView();
      else if(page==='sources')html=await sourcesView();
      else if(page==='suggestions')html=await suggestionsView();
      else html=settingsView();
      if(version===epoch)$('#content').innerHTML=html;
    } catch(error){if(version===epoch){$('#content').innerHTML='<button class="secondary" data-action="refresh">Retry</button>';fail(error);}}
    finally{if(version===epoch)$('#content').setAttribute('aria-busy','false');}
  }
  async function historyView(){
    const params=new URLSearchParams({q:state.query,platform:state.platform,channel:state.channel,after:state.after?Math.floor(new Date(`${state.after}T00:00:00`).getTime()/1000):0,offset:state.offset,mode:state.mode});
    const data=await api(`history?${params}`);
    let prior=null;
    const rows=data.items.map(i=>{const current=day(i.last_opened),heading=current!==prior?`<div class="day-heading"><h2>${esc(current)}</h2></div>`:'';prior=current;return heading+`<button class="history-row" data-item="${i.id}" aria-label="View details: ${esc(i.title||'Untitled')}"><span class="row-time">${time(i.last_opened)}</span><span class="platform-icon">${esc(i.platform.slice(0,1).toUpperCase())}</span><span class="row-main"><span class="row-title">${esc(i.title||'Untitled')}</span><span class="row-sub">${esc(i.channel||'')}${i.kind?` / ${esc(i.kind)}`:''}</span></span><span class="row-meta"><span class="row-platform">${esc(i.platform)}</span><span class="evidence-label"><span class="mobile-time">${time(i.last_opened)} / </span>Opened · ${esc(browser(i.source))}</span></span><span class="row-arrow" aria-hidden="true">›</span></button>`;}).join('');
    return `<form id="search-form"><label class="searchbox"><span class="sr-only">Search history</span><input type="search" name="q" aria-label="Search history" placeholder="Search titles, channels or series" value="${esc(state.query)}"><button type="submit">Search</button></label><div class="filters"><label>Platform<select name="platform" aria-label="Platform"><option value="">All platforms</option>${state.info.platforms.map(p=>`<option ${p===state.platform?'selected':''} value="${esc(p)}">${esc(p)}</option>`).join('')}</select></label><label>Channel<input name="channel" aria-label="Channel" value="${esc(state.channel)}" placeholder="Exact channel name"></label><label>Opened since<input type="date" name="after" aria-label="Opened since" value="${esc(state.after)}"></label><label>Search mode<select name="mode" aria-label="Search mode"><option value="fts" ${state.mode==='fts'?'selected':''}>Words / phrase</option><option value="literal" ${state.mode==='literal'?'selected':''}>Literal text</option></select></label><button class="secondary" type="submit">Apply</button><button class="secondary" type="button" data-action="clear">Clear</button></div></form><div class="result-count">${data.total} items</div>${rows||`<div class="empty"><h2>${state.info.items?'No matching items':'No history yet'}</h2>${state.info.items?'<button class="secondary" data-action="clear">Clear filters</button>':'<button class="primary" data-page="sources">Select sources</button>'}</div>`}<div class="pagination"><button class="secondary" data-action="previous" ${state.offset===0?'disabled':''}>Previous</button><span>${data.total?`${state.offset+1}–${state.offset+data.items.length} of ${data.total}`:'0 items'}</span><button class="secondary" data-action="next" ${state.offset+50>=data.total?'disabled':''}>Next</button></div>`;
  }
  async function sourcesView(){
    const data=await api('sources');state.profiles=data.profiles;
    const available=new Set(data.profiles.map(p=>p.id));state.selected=new Set([...state.selected].filter(id=>available.has(id)));
    return `<section><div class="section-title"><h2>Browser profiles</h2><span>${data.excluded} excluded by configuration</span></div>${data.profiles.map(p=>`<label class="source-row"><input type="checkbox" data-profile="${esc(p.id)}" ${state.selected.has(p.id)?'checked':''} aria-label="Include ${esc(p.browser)} ${esc(p.name)}"><span class="source-info"><strong>${esc(p.browser)}</strong><p>${esc(p.name)}</p></span></label>`).join('')||'<p>No browser profiles found. Check your config.toml browser and profile settings.</p>'}<div class="source-actions"><button class="primary" data-action="collect" ${state.collecting?'disabled':''}>${state.collecting?'Collecting…':'Collect selected'}</button><span id="selected-count">${state.selected.size} selected</span></div><p class="fine-print">Collection reads browser-history snapshots. Browser files are not modified.</p>${[...data.failures,...data.warnings].map(s=>`<p class="fine-print">${esc(s)}</p>`).join('')}</section><section class="automation"><h2>Automatic collection</h2><p>Hourly and logon collection</p><div class="source-actions"><button class="secondary" data-action="schedule-status">Check status</button><button class="secondary" data-action="schedule-install">Enable automatic collection</button><button class="secondary" data-action="schedule-remove">Disable automatic collection</button></div><pre id="schedule-output" role="status"></pre><p class="fine-print">The task uses config.toml, not the selection above.</p></section>`;
  }
  async function suggestionsView(){
    const data=await api(`suggestions?seconds=${state.seconds}${state.seed?`&item=${state.seed}`:''}`);
    const shelf=(title,items,empty)=>`<section class="shelf"><h2>${title}</h2>${items.length?items.map(i=>`<button class="small-row" data-item="${i.id}"><span><strong>${esc(i.title||'Untitled')}</strong><small>${esc(i.platform)} · ${esc(i.reason)}</small></span></button>`).join(''):`<p class="fine-print">${empty}</p>`}</section>`;
    return `<div class="suggestion-controls"><label>Time budget <select id="budget" aria-label="Time budget">${[900,1800,3600].map(n=>`<option value="${n}" ${state.seconds===n?'selected':''}>${n/60} minutes</option>`).join('')}</select></label>${state.seed?'<button class="secondary" data-action="clear-seed">Clear topic selection</button>':''}</div>`+shelf('Rediscover',data.rediscover,'No items last opened more than 30 days ago.')+shelf('Fits your time',data.timebox,'No known video durations fit this budget.')+shelf('Continue watching',data.continue_watching,'No measured playback positions available.')+shelf('Similar by topic',data.similar,state.seed?'No shared catalogue tags.':'Select “Similar by topic” in an item’s details.');
  }
  function settingsView(){return `<section class="settings-section"><h2>Appearance</h2><div class="theme-gallery" role="group" aria-label="Theme">${themes.map(([id,name])=>`<button class="theme-choice" data-theme="${id}" aria-label="${name}" aria-pressed="${state.theme===id}"><span class="theme-swatch swatch-${id}" aria-hidden="true"><i></i><i></i><i></i><i></i></span><strong>${name}</strong><span class="theme-subtitle">${state.theme===id?'Selected':''}</span></button>`).join('')}</div><div class="setting-row"><strong>List spacing</strong><div class="segmented" aria-label="List spacing">${['comfortable','compact'].map(d=>`<button data-density="${d}" aria-pressed="${state.density===d}">${d==='compact'?'Compact':'Comfortable'}</button>`).join('')}</div></div></section><section class="settings-section"><h2>Catalogue</h2><p class="catalogue-path">${esc(state.info.catalogue)}</p><p class="fine-print">Configuration</p><p class="catalogue-path">${esc(state.info.config)}</p><div class="source-actions"><button class="secondary" data-action="export-json">Export JSON</button><button class="secondary" data-action="export-csv">Export CSV</button></div><h2>Playback capture</h2><p>Start Run Playback.cmd, then configure the userscript using the included PLAYBACK.md instructions.</p><div class="source-actions"><button class="secondary" data-action="playback-status">Check playback inbox</button><button class="secondary" data-action="playback-process">Process playback</button></div><p id="playback-output" role="status"></p><p class="fine-print">The server exits after ${state.info.idle_seconds} seconds without requests. Appearance is stored in this browser.</p></section>`;}
  function original(item){
    const id=encodeURIComponent(item.external_id);
    if(item.platform==='bilibili'){const parts=item.external_id.match(/^(.+):p(\d+)$/);return parts?`https://www.bilibili.com/video/${encodeURIComponent(parts[1])}?p=${parts[2]}`:null;}
    if(item.platform==='twitch'){const parts=item.external_id.split(':');return parts[0]==='vod'?`https://www.twitch.tv/videos/${encodeURIComponent(parts[1])}`:parts[0]==='live'?`https://www.twitch.tv/${encodeURIComponent(parts[1])}`:null;}
    const base={youtube:`https://www.youtube.com/watch?v=${id}`,netflix:`https://www.netflix.com/watch/${id}`,prime:`https://www.primevideo.com/detail/${id}`,disney:`https://www.disneyplus.com/video/${id}`,crunchyroll:`https://www.crunchyroll.com/watch/${id}`};
    return base[item.platform]||null;
  }
  async function showItem(id,button){
    try{const i=await api(`item?id=${id}`);currentItem=i;opener=button;
      const link=original(i);
      $('#detail').innerHTML=`<div class="sheet-top"><span>Details</span><button class="icon-button" data-action="close" aria-label="Close details">×</button></div><h2 id="detail-title">${esc(i.title||'Untitled')}</h2><p class="detail-subtitle">${esc(i.channel||i.platform)}</p><dl class="detail-list"><div><dt>Platform</dt><dd>${esc(i.platform)}</dd></div><div><dt>Type</dt><dd>${esc(i.kind||'Unknown')}</dd></div><div><dt>Video duration</dt><dd>${duration(i.duration_s)}</dd></div><div><dt>Recorded playback</dt><dd>${duration(i.played_seconds)}</dd></div><div><dt>Openings</dt><dd>${i.visit_count}</dd></div></dl><div class="tags">${i.tags.map(t=>`<span>${esc(t)}</span>`).join('')}</div><div class="detail-actions">${link?`<a class="primary open-original" href="${esc(link)}" target="_blank" rel="noopener noreferrer">Open original</a>`:'<p class="fine-print">Local files are not opened by this interface.</p>'}<button class="secondary" data-action="similar">Similar by topic</button><button class="secondary" data-action="edit">Edit metadata and tags</button><button class="danger-button" data-action="delete">Delete item</button></div><section class="visits"><h3>Recent openings</h3><p class="fine-print">Tab time is an approximation, not measured playback. Showing up to 50 openings.</p>${i.visits.map(v=>`<div class="visit"><span>${esc(date(v.at))}<br>${esc(browser(v.source))}</span><span>Tab: ${duration(v.tab_seconds)}</span></div>`).join('')}</section>`;
      $('#detail').showModal();$('#detail [data-action="close"]').focus();
    }catch(error){fail(error);}
  }
  function ask(title,text,label,action){confirmed=action;$('#confirmation').innerHTML=`<h2 id="confirm-title">${esc(title)}</h2><p>${esc(text)}</p><div class="confirm-actions"><button class="secondary" data-action="cancel">Cancel</button><button class="primary" data-action="confirm">${esc(label)}</button></div>`;$('#confirmation').showModal();$('#confirmation [data-action="cancel"]').focus();}
  async function refresh(){message();await info();await render();}
  async function exportFile(ext){
    let handle=null;
    if(window.showSaveFilePicker){try{handle=await showSaveFilePicker({suggestedName:`omoi-history.${ext}`});}catch(e){if(e.name==='AbortError')return;throw e;}}
    const response=await fetch(`/api/export.${ext}`,{headers:{'X-Omoi-Token':token}});
    if(!response.ok)throw new Error('Export failed.');
    if(handle){const writer=await handle.createWritable();await response.body.pipeTo(writer);}
    else{if(Number(response.headers.get('Content-Length'))>64*1024*1024){await response.body.cancel();throw new Error('Use the CLI for exports above 64 MB in this browser.');}const url=URL.createObjectURL(await response.blob());const a=document.createElement('a');a.href=url;a.download=`omoi-history.${ext}`;a.click();setTimeout(()=>URL.revokeObjectURL(url),30000);}
    message('Export saved.');
  }
  async function schedule(action){const result=await api('schedule',{action});const target=$('#schedule-output');if(target)target.textContent=result.output;}
  const actions={
    'export-json'(){return exportFile('json');},'export-csv'(){return exportFile('csv');},
    'schedule-status'(){return schedule('status');},
    'schedule-install'(){ask('Enable automatic collection?','Install the hourly and logon task for profiles in config.toml? Keep both executables in their current folder. This applies to all configured profiles.','Enable',()=>schedule('install'));},
    'schedule-remove'(){ask('Disable automatic collection?','Remove the Omoi collection task? Your catalogue remains available.','Disable',()=>schedule('remove'));},
    async 'playback-status'(){const p=await api('playback');$('#playback-output').textContent=`${p.pending} events pending; ${p.dropped} dropped. Listener port: ${p.port}. Token file: ${p.token_file}`;},
    async 'playback-process'(){const p=await api('playback/process',{});await info();$('#playback-output').textContent=`${p.processed} events processed.`;},
    edit(){const i=currentItem;$('#detail').innerHTML=`<div class="sheet-top"><h2 id="detail-title">Edit item</h2><button class="icon-button" data-action="close" aria-label="Close details">×</button></div><form id="edit-form" class="edit-form"><label>Title<input name="title" maxlength="4096" value="${esc(i.title)}"></label><label>Channel<input name="channel" maxlength="1024" value="${esc(i.channel)}"></label><label>Type<input name="kind" maxlength="64" value="${esc(i.kind)}" placeholder="Documentary, tutorial, music…"></label><label>Duration in seconds<input name="duration" type="number" min="0" max="31536000" value="${i.duration_s??''}"></label><label>Tags, one per line<textarea name="tags" rows="5">${esc(i.tags.join('\n'))}</textarea></label><p id="edit-error" role="alert"></p><button class="primary" type="submit">Save changes</button></form>`;$('#edit-form input').focus();},
    refresh,
    clear(){Object.assign(state,{query:'',platform:'',channel:'',after:'',offset:0});return render();},
    previous(){state.offset=Math.max(0,state.offset-50);return render();},next(){state.offset+=50;return render();},
    close(){$('#detail').close();},cancel(){$('#confirmation').close();confirmed=null;},
    async confirm(){const action=confirmed;confirmed=null;$('#confirmation').close();if(action)await action();},
    delete(){const id=currentItem.id;ask('Delete this item?','This permanently removes the item, its openings, playback records and tags from the catalogue. Browser history is unchanged. A future opening may add the item again.','Delete item',async()=>{await api('delete',{id});$('#detail').close();if(state.seed===id)state.seed=null;state.offset=0;await refresh();message('Item deleted.');});},
    similar(){state.seed=currentItem.id;$('#detail').close();state.page='suggestions';return render();},
    'clear-seed'(){state.seed=null;return render();},
    collect(){if(state.collecting)return;if(!state.selected.size){message('Select at least one profile.',true);return;}const profiles=[...state.selected];ask('Collect selected profiles?',`Read history from ${profiles.length} selected profile(s) and add video openings to this catalogue?`,'Collect',async()=>{state.collecting=true;message('Collecting selected profiles…');document.querySelectorAll('[data-action="collect"]').forEach(b=>b.disabled=true);try{const report=await api('collect',{profiles});await info();await render();message(`${report.items_inserted} items added; ${report.views_inserted} openings added; ${report.profiles_failed} profiles failed.${report.warnings.length?' '+report.warnings.join('; '):''}`,report.profiles_failed>0);}finally{state.collecting=false;document.querySelectorAll('[data-action="collect"]').forEach(b=>{b.disabled=false;b.textContent='Collect selected';});}});}
  };
  document.addEventListener('click',async event=>{const b=event.target.closest('button');if(!b||b.disabled)return;try{
    if(b.dataset.page){state.page=b.dataset.page;message();await render();window.scrollTo(0,0);$('#main').focus();}
    else if(b.dataset.item)await showItem(Number(b.dataset.item),b);
    else if(b.dataset.theme){state.theme=b.dataset.theme;applyTheme();savePreferences();$('#content').innerHTML=settingsView();$(`[data-theme="${state.theme}"]`).focus();}
    else if(b.dataset.density){state.density=b.dataset.density;applyTheme();savePreferences();$('#content').innerHTML=settingsView();$(`[data-density="${state.density}"]`).focus();}
    else if(b.dataset.action)await actions[b.dataset.action]?.();
  }catch(error){fail(error);}});
  document.addEventListener('submit',async event=>{if(event.target.id!=='edit-form')return;event.preventDefault();const form=event.target;const data=new FormData(form);const button=form.querySelector('button[type="submit"]');button.disabled=true;try{await api('edit',{id:currentItem.id,title:data.get('title'),channel:data.get('channel'),kind:data.get('kind'),duration_s:data.get('duration')===''?null:Number(data.get('duration')),tags:String(data.get('tags')).split('\n').map(t=>t.trim()).filter(Boolean)});$('#detail').close();await refresh();message('Item saved.');}catch(e){$('#edit-error').textContent=e.message;}finally{button.disabled=false;}});
  document.addEventListener('submit',event=>{if(event.target.id!=='search-form')return;event.preventDefault();const data=new FormData(event.target);for(const key of ['q','platform','channel','after','mode'])state[key==='q'?'query':key]=data.get(key);state.offset=0;render();});
  document.addEventListener('change',event=>{const e=event.target;if(e.dataset.profile){e.checked?state.selected.add(e.dataset.profile):state.selected.delete(e.dataset.profile);$('#selected-count').textContent=`${state.selected.size} selected`;}else if(e.id==='budget'){state.seconds=Number(e.value);render();}});
  $('.brand').addEventListener('click',event=>{event.preventDefault();state.page='history';render();});
  $('#detail').addEventListener('close',()=>{if(opener?.isConnected)opener.focus();else $('#main').focus();});
  document.addEventListener('keydown',event=>{
    if(event.key!=='Tab')return;
    const dialog=$('#confirmation').open?$('#confirmation'):$('#detail').open?$('#detail'):null;if(!dialog)return;
    const controls=[...dialog.querySelectorAll('button:not(:disabled),a[href],input:not(:disabled),textarea:not(:disabled),select:not(:disabled),[tabindex="0"]')].filter(e=>e.getClientRects().length);
    if(!controls.length){event.preventDefault();return;}
    const first=controls[0],last=controls.at(-1),active=document.activeElement;
    if(!dialog.contains(active)||(!event.shiftKey&&active===last)){event.preventDefault();first.focus();}
    else if(event.shiftKey&&active===first){event.preventDefault();last.focus();}
  },true);
  document.addEventListener('keydown',event=>{if(event.key==='/'&&!['INPUT','TEXTAREA','SELECT'].includes(document.activeElement.tagName)&&!$('#detail').open&&!$('#confirmation').open){event.preventDefault();$('input[name="q"]')?.focus();}});
  async function start(){try{if(!/^[a-f0-9]{64}$/.test(token))throw new Error('Open the launch URL printed by omoi ui.');await info();try{const saved=JSON.parse(localStorage.getItem(preferenceKey())||'{}');if(themes.some(t=>t[0]===saved.theme))state.theme=saved.theme;if(['comfortable','compact'].includes(saved.density))state.density=saved.density;}catch{/* Browser storage is optional. */}applyTheme();await render();}catch(error){$('#content').innerHTML='';$('#content').setAttribute('aria-busy','false');fail(error);}}
  start();
})();
