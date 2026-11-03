/* Shared packaged capture logic. No page-world bridge, network or storage access. */
globalThis.OmoiCapture = { install(adapter) {
  const uuid=()=>crypto.randomUUID();
  const attached=new WeakSet();
  function attach(video){
    if(attached.has(video))return;attached.add(video);
    let producer=uuid(),seq=0,url=location.href,total=0,last=performance.now(),position=video.currentTime,rate=video.playbackRate,playing=false,sent=-Infinity,paired=adapter.epoch();
    function sample(event){
      const now=performance.now(),at=Date.now(),current=video.currentTime;
      if(location.href!==url||paired!==adapter.epoch()){producer=uuid();seq=0;url=location.href;total=0;playing=false;sent=-Infinity;paired=adapter.epoch();}
      // Ads and unpaired intervals must not enter the accumulator at all.
      if(video.closest('.ad-showing')||!adapter.enabled()){last=now;position=current;rate=video.playbackRate;playing=false;return;}
      const wall=now-last,movement=(current-position)*1000/Math.max(rate,0.01);
      if(playing&&!video.seeking&&event.type!=='seeking'&&wall>=0&&wall<=5000&&movement>=0&&movement<=wall+1500)total+=Math.max(0,Math.min(wall,movement));
      last=now;position=current;rate=video.playbackRate;
      playing=!video.paused&&!video.ended&&!video.seeking&&video.readyState>=3&&!['waiting','stalled'].includes(event.type);
      if(event.type==='timeupdate'&&now-sent<5000)return;sent=now;
      const channel=document.querySelector('#owner #channel-name a, [itemprop="author"] [itemprop="name"]')?.textContent?.trim();
      adapter.enqueue({event_id:`${producer}:${seq}`,producer_id:producer,seq:seq++,url,at_ms:at,state:video.ended?'stopped':playing?'playing':'paused',position_ms:Math.max(0,Math.floor(current*1000)||0),cumulative_played_ms:Math.floor(total),title:document.title.slice(0,1024),channel:channel?.slice(0,256)||null,duration_s:Number.isFinite(video.duration)?Math.floor(video.duration):null});
    }
    for(const type of ['playing','pause','ended','waiting','stalled','seeking','seeked','ratechange','timeupdate'])video.addEventListener(type,sample);
    sample({type:'attach'});
  }
  function scan(){document.querySelectorAll('video').forEach(attach);}
  new MutationObserver(records=>{if(records.some(r=>r.addedNodes.length))scan();}).observe(document.documentElement,{childList:true,subtree:true});
  scan();
}};
