const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
function captureFixture(reply=200){
 const handlers={},menus={},requests=[],timers=new Map(),storage=new Map([['token','a'.repeat(64)]]);
 let now=0,uuid=0,timerId=0,ad=false;
 const video={currentTime:0,paused:true,ended:false,seeking:false,readyState:4,playbackRate:1,duration:600,addEventListener:(t,f)=>handlers[t]=f,closest:()=>ad?{}:null};
 const context={crypto:{randomUUID:()=>`id-${++uuid}`},performance:{now:()=>now},Date:{now:()=>1700000000000+now},location:{href:'https://www.youtube.com/watch?v=abcdefghijk'},document:{title:'Fixture',documentElement:{},querySelector:()=>null,querySelectorAll:()=>[video]},MutationObserver:class{observe(){}},GM_registerMenuCommand:(name,fn)=>menus[name]=fn,GM_getValue:(k,d)=>storage.has(k)?structuredClone(storage.get(k)):d,GM_setValue:(k,v)=>storage.set(k,structuredClone(v)),GM_deleteValue:k=>storage.delete(k),GM_listValues:()=>[...storage.keys()],GM_xmlhttpRequest:r=>{requests.push(r);if(reply!==null)r.onload({status:reply});return {abort(){}};},setTimeout:f=>{timers.set(++timerId,f);return timerId;},clearTimeout:id=>timers.delete(id),prompt:(_text,defaultValue)=>defaultValue||'a'.repeat(64),alert:()=>{}};
 vm.runInNewContext(fs.readFileSync('userscripts/omoi.user.js','utf8'),context);
 return {video,context,menus,requests,timers,storage,setAd:value=>{ad=value;},reply:value=>{reply=value;},emit(type='timeupdate',ms=0,move=ms/1000){now+=ms;video.currentTime+=move;handlers[type]({type});},last:()=>JSON.parse(requests.at(-1).data)};
}

test('stop during backoff can pair and send again',()=>{
 const f=captureFixture(503);
 f.menus['Stop capture and clear pending events']();
 f.reply(200);f.menus['Pair Omoi']();f.video.paused=false;f.emit('playing');
 assert.ok(f.requests.length>=2,'new pairing must not remain stuck behind a cancelled retry');
});
test('ad intervals never enter cumulative content playback',()=>{
 const f=captureFixture();f.video.paused=false;f.emit('playing');
 for(let i=0;i<5;i++)f.emit('timeupdate',1000);
 f.setAd(true);f.emit('playing');
 for(let i=0;i<10;i++)f.emit('timeupdate',1000);
 f.setAd(false);f.emit('playing');
 for(let i=0;i<5;i++)f.emit('timeupdate',1000);
 f.video.paused=true;f.emit('pause');
 assert.equal(f.last().cumulative_played_ms,10000);
});
test('re-pairing starts a fresh producer and excludes unpaired time',()=>{
 const f=captureFixture();f.video.paused=false;f.emit('playing');f.emit('timeupdate',5000);
 const prior=f.last().producer_id;
 f.menus['Stop capture and clear pending events']();
 f.emit('timeupdate',5000);f.emit('timeupdate',5000);
 f.menus['Pair Omoi']();f.emit('playing');
 assert.notEqual(f.last().producer_id,prior);
 assert.equal(f.last().cumulative_played_ms,0);
});
test('permanently rejected event does not block the transport queue',()=>{
 const f=captureFixture(400);f.reply(200);f.video.paused=false;f.emit('playing');
 assert.ok(f.requests.length>=2);
 assert.notEqual(JSON.parse(f.requests[0].data).event_id,f.last().event_id);
 assert.equal(f.storage.get('dropped'),1);
});
test('rate change accounts preceding interval at its previous rate',()=>{
 const f=captureFixture();f.video.paused=false;f.emit('playing');
 f.video.playbackRate=2;f.emit('ratechange',1000,1);
 assert.equal(f.last().cumulative_played_ms,1000);
});
test('new SPA identity emits its baseline despite the previous throttle',()=>{
 const f=captureFixture();f.video.paused=false;f.emit('playing');
 const count=f.requests.length;
 f.context.location.href='https://www.youtube.com/watch?v=12345678901';
 f.emit('timeupdate',1000,0);
 assert.equal(f.requests.length,count+1);
 assert.equal(f.last().seq,0);
 assert.equal(f.last().cumulative_played_ms,0);
});
test('capture excludes pause, seeking and silence; retries preserve IDs',()=>{
 const handlers={},events=[],storage=new Map([['token','a'.repeat(64)]]);let now=0,uuid=0,fail=true,timer;
 const video={currentTime:0,paused:true,ended:false,seeking:false,readyState:4,playbackRate:1,duration:600,addEventListener:(t,f)=>handlers[t]=f,closest:()=>null};
 const context={crypto:{randomUUID:()=>`id-${++uuid}`},performance:{now:()=>now},Date:{now:()=>1700000000000+now},location:{href:'https://www.youtube.com/watch?v=abcdefghijk'},document:{title:'Fixture',documentElement:{},querySelector:()=>null,querySelectorAll:()=>[video]},MutationObserver:class{observe(){}},GM_registerMenuCommand:()=>{},GM_getValue:(k,d)=>storage.has(k)?storage.get(k):d,GM_setValue:(k,v)=>storage.set(k,structuredClone(v)),GM_deleteValue:k=>storage.delete(k),GM_listValues:()=>[...storage.keys()],GM_xmlhttpRequest:r=>{events.push(JSON.parse(r.data));if(fail){fail=false;r.onerror();}else r.onload({status:200});},setTimeout:f=>{timer=f;return 1;},clearTimeout:()=>{timer=null;}};
 vm.runInNewContext(fs.readFileSync('userscripts/omoi.user.js','utf8'),context);
 timer();assert.equal(events[0].event_id,events[1].event_id);
 video.paused=false;handlers.playing({type:'playing'});
 for(let n=0;n<30;n++){now+=1000;video.currentTime++;handlers.timeupdate({type:'timeupdate'});}
 video.paused=true;handlers.pause({type:'pause'});now+=60000;
 video.currentTime=400;handlers.seeking({type:'seeking'});handlers.seeked({type:'seeked'});
 video.paused=false;handlers.playing({type:'playing'});
 for(let n=0;n<30;n++){now+=1000;video.currentTime++;handlers.timeupdate({type:'timeupdate'});}
 video.paused=true;handlers.pause({type:'pause'});
 assert.equal(events.at(-1).cumulative_played_ms,60000);
 assert.equal(events.at(-1).position_ms,430000);
 assert.ok([...storage.keys()].every(k=>!k.startsWith('queue:')));
});
