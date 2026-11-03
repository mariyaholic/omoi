// ==UserScript==
// @name         Omoi playback
// @namespace    omoi.local
// @version      0.2.0
// @description  Send opted-in video playback samples to your local Omoi listener.
// @match        https://www.youtube.com/*
// @match        https://www.netflix.com/*
// @match        https://www.twitch.tv/*
// @match        https://www.bilibili.com/*
// @match        https://www.primevideo.com/*
// @match        https://www.disneyplus.com/*
// @match        https://www.crunchyroll.com/*
// @grant        GM_getValue
// @grant        GM_setValue
// @grant        GM_deleteValue
// @grant        GM_listValues
// @grant        GM_xmlhttpRequest
// @grant        GM_registerMenuCommand
// @connect      127.0.0.1
// @noframes
// @run-at       document-idle
// ==/UserScript==
/* Capture requires explicit pairing. No cookies, page-world code or external requests. */
(() => {
  'use strict';
  const uuid=()=>crypto.randomUUID();
  const queueKey=`queue:${uuid()}`;
  let sending=false,retry=null,backoff=1000,request=null;
  const pairing=()=>`${GM_getValue('token','')}:${GM_getValue('pairing','')}:${GM_getValue('server-generation','')}`;
  function resetTransport(){
    const prior=request;request=null;sending=false;
    if(retry!==null)clearTimeout(retry);
    retry=null;backoff=1000;prior?.abort();
  }
  GM_registerMenuCommand('Pair Omoi',()=>{
    const token=prompt('Paste the contents of the Omoi listener token file.');
    if(!token)return;
    if(!/^[a-f0-9]{64}$/.test(token.trim())){alert('Expected 64 hexadecimal characters.');return;}
    const port=Number(prompt('Listener port','43821'));
    if(!Number.isInteger(port)||port<1024||port>65535)return;
    resetTransport();GM_setValue('pairing',uuid());GM_setValue('token',token.trim());GM_setValue('port',port);hello();flush();
  });
  GM_registerMenuCommand('Stop capture and clear pending events',()=>{
    GM_deleteValue('token');for(const k of GM_listValues().filter(k=>k.startsWith('queue:')))GM_deleteValue(k);
    resetTransport();
  });
  let helloAt=-Infinity;
  function hello(){
    const token=GM_getValue('token','');if(!token||Date.now()-helloAt<30000)return;
    helloAt=Date.now();const pair=pairing();
    GM_xmlhttpRequest({method:'POST',url:`http://127.0.0.1:${GM_getValue('port',43821)}/hello`,anonymous:true,headers:{'Content-Type':'application/json','X-Omoi-Token':token},data:JSON.stringify({protocol:1,version:'0.2.0',permission:true}),timeout:5000,
      onload:r=>{if(pair!==pairing()||r.status!==200)return;try{const h=JSON.parse(r.responseText);if(h.compatible&&Number.isSafeInteger(h.generation))GM_setValue('server-generation',h.generation);}catch{}},onerror:()=>{},ontimeout:()=>{}});
  }
  function enqueue(e){
    hello();
    if(!GM_getValue('token',''))return;
    const keys=GM_listValues().filter(k=>k.startsWith('queue:'));
    if(keys.length>=32&&!keys.includes(queueKey)){GM_setValue('dropped',(GM_getValue('dropped',0)||0)+1);return;}
    const q=GM_getValue(queueKey,[]);q.push(e);
    if(q.length>256){q.shift();GM_setValue('dropped',(GM_getValue('dropped',0)||0)+1);}
    GM_setValue(queueKey,q);flush();
  }
  function flush(){
    if(sending||retry)return;
    const token=GM_getValue('token','');if(!token)return;
    const key=GM_listValues().filter(k=>k.startsWith('queue:')).sort()[0];if(!key)return;
    const q=GM_getValue(key,[]);if(!q.length){GM_deleteValue(key);flush();return;}
    sending=true;
    const attempt={pair:pairing(),abort:()=>{}};request=attempt;
    const accept=()=>{if(request!==attempt)return false;request=null;sending=false;if(attempt.pair!==pairing()){flush();return false;}return true;};
    const failed=()=>{if(!accept())return;const id=setTimeout(()=>{if(retry!==id)return;retry=null;flush();},backoff);retry=id;backoff=Math.min(60000,backoff*2);};
    try { const handle=GM_xmlhttpRequest({method:'POST',url:`http://127.0.0.1:${GM_getValue('port',43821)}/event`,anonymous:true,
      headers:{'Content-Type':'application/json','X-Omoi-Token':token},data:JSON.stringify(q[0]),timeout:5000,
      onload:r=>{if(r.status!==200&&r.status!==400){failed();return;}if(!accept())return;backoff=1000;if(r.status===400)GM_setValue('dropped',(GM_getValue('dropped',0)||0)+1);const latest=GM_getValue(key,[]).filter(e=>e.event_id!==q[0].event_id);if(latest.length)GM_setValue(key,latest);else GM_deleteValue(key);flush();},onerror:failed,ontimeout:failed});attempt.abort=()=>handle?.abort?.();}catch{failed();}
  }
  OmoiCapture.install({epoch:pairing,enabled:()=>!!GM_getValue('token',''),enqueue});
  hello();flush();
})();
