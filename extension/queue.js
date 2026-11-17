/* Only the service worker writes this queue. Stored events retain their IDs. */
globalThis.OmoiQueue = {
  append(state, entry) {
    if(state.queue.some(x=>x.event.event_id===entry.event.event_id))return true;
    const bytes=new TextEncoder().encode(JSON.stringify(entry)).length;
    if(bytes>20000||state.queue.length>=1024||new TextEncoder().encode(JSON.stringify(state.queue)).length+bytes>4*1024*1024)return false;
    state.queue.push(entry);return true;
  },
  acknowledge(state, id) { state.queue=state.queue.filter(x=>x.event.event_id!==id); },
  epoch(state, binding, generation) {
    if(state.binding&&state.binding!==binding)throw new Error('Catalogue changed. Disconnect before connecting to a different catalogue.');
    const retained=state.queue.filter(x=>x.binding===binding&&x.generation===generation);
    state.dropped+=state.queue.length-retained.length;state.queue=retained;
    state.binding=binding;state.generation=generation;
  }
};
