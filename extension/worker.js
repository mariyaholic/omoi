'use strict';
importScripts('queue.js');
const HOST='org.omoi.capture_dev';
const sites=chrome.runtime.getManifest().optional_host_permissions;
let port=null,inflight=null,portTimer=null,chain=Promise.resolve(),pending=0,helloAt=0,lastError='';
chrome.storage.local.setAccessLevel({accessLevel:'TRUSTED_CONTEXTS'});
async function load(){const {capture}=await chrome.storage.local.get('capture');return capture||{enabled:false,binding:null,generation:0,queue:[],dropped:0};}
async function save(s){await chrome.storage.local.set({capture:s});}
function close(){if(port){const p=port;port=null;p.disconnect();}if(portTimer)clearTimeout(portTimer);portTimer=null;helloAt=0;}
function exchange(message){
  return new Promise((resolve,reject)=>{
    if(inflight){reject(new Error('Native request already active'));return;}
    if(!port){
      port=chrome.runtime.connectNative(HOST);const owned=port;
      owned.onMessage.addListener(reply=>{if(port!==owned||!inflight)return;const done=inflight;inflight=null;clearTimeout(done.timer);done.resolve(reply);});
      owned.onDisconnect.addListener(()=>{if(port!==owned)return;port=null;helloAt=0;if(inflight){const done=inflight;inflight=null;clearTimeout(done.timer);done.reject(new Error(chrome.runtime.lastError?.message||'Native host disconnected'));}});
    }
    const timer=setTimeout(()=>{if(!inflight)return;inflight=null;close();reject(new Error('Native host timed out'));},5000);
    inflight={resolve,reject,timer};
    if(portTimer)clearTimeout(portTimer);portTimer=setTimeout(close,30000);
    try{port.postMessage(message);}catch(e){inflight=null;clearTimeout(timer);close();reject(e);}
  });
}
async function negotiate(s){
  if(!s.enabled)return false;
  if(port&&Date.now()-helloAt<15000)return true;
  const permission=await chrome.permissions.contains({origins:sites});
  const reply=await exchange({type:'hello',protocol_min:1,protocol_max:1,version:'0.2.0',permission,binding:s.binding});
  if(!reply.ok){lastError=reply.error;helloAt=0;return false;}
  OmoiQueue.epoch(s,reply.binding,reply.generation);await save(s);helloAt=Date.now();lastError='';return true;
}
async function flush(s){
  if(!s.queue.length||!await negotiate(s))return;
  for(let n=0;n<64&&s.queue.length;n++){
    const head=s.queue[0];
    if(head.binding!==s.binding||head.generation!==s.generation)break;
    const reply=await exchange({type:'event',...head});
    if(!reply.ok){lastError=reply.error;helloAt=0;if(reply.drop){OmoiQueue.acknowledge(s,head.event.event_id);s.dropped++;await save(s);continue;}break;}
    OmoiQueue.acknowledge(s,head.event.event_id);await save(s);
  }
  if(s.queue.length)await chrome.alarms.create('retry',{delayInMinutes:1});
}
function trustedContent(sender){try{const u=new URL(sender.url);return sender.id===chrome.runtime.id&&sender.frameId===0&&sites.some(s=>s===`${u.origin}/*`);}catch{return false;}}
async function handle(message,sender){
  const popup=sender.id===chrome.runtime.id&&sender.url===chrome.runtime.getURL('popup.html');
  if(!popup&&!trustedContent(sender))throw new Error('Sender forbidden');
  let s=await load();
  if(message.type==='disconnect'&&popup){s.enabled=false;s.queue=[];s.binding=null;s.generation=0;await save(s);close();return {enabled:false};}
  if(message.type==='connect'&&popup){s.enabled=true;await save(s);helloAt=0;}
  if(!['connect','state','event'].includes(message.type))throw new Error('Unknown action');
  // A known catalogue/epoch can durably queue while its host is temporarily
  // unavailable. It can never move to a different home implicitly.
  if(message.type==='event'){
    if(!s.enabled||!s.binding||message.epoch!==`${s.binding}:${s.generation}`)return {accepted:false,enabled:false,error:'Reconnect capture'};
    const e=message.event;
    if(!e||new URL(e.url).origin!==new URL(sender.url).origin||typeof e.event_id!=='string'||JSON.stringify(e).length>16384)throw new Error('Invalid event');
    if(!OmoiQueue.append(s,{binding:s.binding,generation:s.generation,event:e}))return {accepted:false,enabled:true,error:'Local queue full; retry later'};
    await save(s); // Content ack only after durable browser storage.
  }
  let connected=false;
  try{connected=await negotiate(s);await flush(s);}catch(e){lastError=e.message;await chrome.alarms.create('retry',{delayInMinutes:1});}

  return {accepted:message.type==='event',enabled:connected&&s.enabled,epoch:`${s.binding}:${s.generation}`,pending:s.queue.length,dropped:s.dropped,error:lastError};
}
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(pending>=128){respond({accepted:false,error:'Busy; retry'});return;}
  pending++;
  chain=chain.catch(()=>{}).then(()=>handle(message,sender)).then(respond,e=>{lastError=e.message;respond({accepted:false,enabled:false,error:lastError});chrome.alarms.create('retry',{delayInMinutes:1});}).finally(()=>pending--);
  return true;
});
chrome.alarms.onAlarm.addListener(alarm=>{if(alarm.name==='retry')chain=chain.catch(()=>{}).then(async()=>{const s=await load();await flush(s);}).catch(e=>{lastError=e.message;chrome.alarms.create('retry',{delayInMinutes:1});});});
