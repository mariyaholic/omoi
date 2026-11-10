'use strict';
(() => {
  let enabled=false,epoch='',queue=[],sending=false,retry=null;
  async function state(){try{const s=await chrome.runtime.sendMessage({type:'state'});if(s.epoch&&epoch!==s.epoch){queue=[];epoch=s.epoch;}enabled=!!s.enabled;}catch{enabled=false;}}
  async function flush(){
    if(sending||!queue.length)return;sending=true;const event=queue[0],owned=epoch;
    try{const s=await chrome.runtime.sendMessage({type:'event',epoch:owned,event});if(owned!==epoch)return;if(s.accepted)queue.shift();if(s.enabled===false){enabled=false;queue=[];await state();}}
    catch{enabled=false;}
    finally{sending=false;if(queue.length&&!retry)retry=setTimeout(()=>{retry=null;flush();},2000);}
  }
  OmoiCapture.install({epoch:()=>epoch,enabled:()=>enabled,enqueue:event=>{if(queue.length<64){queue.push(event);flush();}}});
  async function poll(){await state();flush();setTimeout(poll,15000);}
  poll();
})();
