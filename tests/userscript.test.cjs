const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
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
