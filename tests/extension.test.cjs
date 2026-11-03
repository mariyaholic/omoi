const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const context={TextEncoder};vm.runInNewContext(fs.readFileSync('extension/queue.js','utf8'),context);const Q=context.OmoiQueue;
function state(){return {queue:[],binding:'home-a',generation:1,dropped:0};}
function e(n,generation=1){return {binding:'home-a',generation,event:{event_id:`producer:${n}`,seq:n}};}
test('worker queue preserves IDs across lost acknowledgements and restart',()=>{let s=state();Q.append(s,e(1));Q.append(s,e(1));assert.equal(s.queue.length,1);s=JSON.parse(JSON.stringify(s));Q.append(s,e(2));Q.acknowledge(s,'producer:1');assert.equal(s.queue[0].event.event_id,'producer:2');});
test('consent epochs cannot replay old capture or cross catalogues',()=>{const s=state();Q.append(s,e(1));Q.epoch(s,'home-a',2);assert.equal(s.queue.length,0);assert.equal(s.dropped,1);assert.throws(()=>Q.epoch(s,'home-b',2));});
test('queue is bounded and rejects overflow without deleting an unacknowledged event',()=>{const s=state();for(let i=0;i<1024;i++)assert.equal(Q.append(s,e(i)),true);assert.equal(Q.append(s,e(1024)),false);assert.equal(s.queue[0].event.event_id,'producer:0');});
test('manifest packages scripts without page messaging or general script loading',()=>{const m=JSON.parse(fs.readFileSync('extension/manifest.json'));assert.equal(m.manifest_version,3);assert.ok(!m.permissions.includes('userScripts'));assert.ok(!m.host_permissions);assert.equal(m.optional_host_permissions.length,7);assert.ok(!m.externally_connectable);assert.equal(fs.readFileSync('extension/core.js','utf8'),fs.readFileSync('capture/core.js','utf8'));});
function workerFixture(storage={}){
 const listeners={},timers=new Map();let timer=0,permission=true,online=true,generation=1,commits=new Set(),wire=[];
 const chrome={storage:{local:{setAccessLevel:async()=>{},get:async()=>structuredClone(storage),set:async value=>{await Promise.resolve();Object.assign(storage,structuredClone(value));}}},permissions:{contains:async()=>permission},alarms:{create:async()=>{},onAlarm:{addListener:fn=>listeners.alarm=fn}},runtime:{id:'a'.repeat(32),getURL:p=>`chrome-extension://${'a'.repeat(32)}/${p}`,getManifest:()=>({optional_host_permissions:['https://www.youtube.com/*']}),onMessage:{addListener:fn=>listeners.message=fn},connectNative:()=>{
  const callbacks={};return {onMessage:{addListener:fn=>callbacks.message=fn},onDisconnect:{addListener:fn=>callbacks.disconnect=fn},disconnect:()=>callbacks.disconnect?.(),postMessage:m=>{
    wire.push(structuredClone(m));queueMicrotask(()=>{
     if(!online){callbacks.disconnect();return;}
     if(m.type==='hello'){callbacks.message(m.permission?{ok:true,binding:'fixture-home',generation}:{ok:false,error:'Permission needed'});return;}
     if(m.type==='event'){if(m.generation!==generation){callbacks.message({ok:false,error:'Reconnect'});return;}commits.add(m.event.event_id);callbacks.message({ok:true});}
    });
  }};
 }}};
 const c={chrome,URL,Date,TextEncoder,console,setTimeout:fn=>{timers.set(++timer,fn);return timer;},clearTimeout:id=>timers.delete(id)};c.importScripts=p=>vm.runInContext(fs.readFileSync('extension/'+p,'utf8'),context);const context=vm.createContext(c);vm.runInContext(fs.readFileSync('extension/worker.js','utf8'),context);
 const popup={id:chrome.runtime.id,url:chrome.runtime.getURL('popup.html')};
 const sender={id:chrome.runtime.id,url:'https://www.youtube.com/watch?v=abcdefghijk',frameId:0};
 const send=(message,from=sender)=>new Promise(resolve=>listeners.message(message,from,resolve));
 return {send,storage,commits,wire,popup,online:x=>online=x,permission:x=>permission=x,generation:x=>generation=x};
}
test('actual worker serializes concurrent tabs and restores durable queue after suspension',async()=>{
 const storage={},f=workerFixture(storage);const ready=await f.send({type:'connect'},f.popup);assert.equal(ready.enabled,true);
 f.online(false);
 const event=i=>({event_id:`tab:${i}`,url:'https://www.youtube.com/watch?v=abcdefghijk'});
 const replies=await Promise.all([1,2,3,4].map(i=>f.send({type:'event',epoch:ready.epoch,event:event(i)})));
 assert.ok(replies.every(x=>x.accepted));assert.equal(storage.capture.queue.length,4);
 const resumed=workerFixture(storage);await resumed.send({type:'state'});assert.equal(storage.capture.queue.length,0);assert.equal(resumed.commits.size,4);
});
test('actual worker refuses permission denial and content commands from untrusted frames',async()=>{
 const f=workerFixture();f.permission(false);assert.equal((await f.send({type:'connect'},f.popup)).enabled,false);assert.equal(f.storage.capture.queue.length,0);
 const bad=await f.send({type:'connect'},{id:'foreign',url:'https://www.youtube.com/',frameId:0});assert.match(bad.error,/forbidden/);
});
test('actual worker consent reset discards old epoch explicitly and rejects stale producers',async()=>{
 const f=workerFixture(),ready=await f.send({type:'connect'},f.popup);f.online(false);
 await f.send({type:'event',epoch:ready.epoch,event:{event_id:'old:0',url:'https://www.youtube.com/watch?v=abcdefghijk'}});
 f.generation(2);f.online(true);const resumed=await f.send({type:'connect'},f.popup);assert.notEqual(resumed.epoch,ready.epoch);assert.equal(resumed.dropped,1);
 assert.equal((await f.send({type:'event',epoch:ready.epoch,event:{event_id:'old:1',url:'https://www.youtube.com/watch?v=abcdefghijk'}})).accepted,false);
});
