// Runs only for the explicit hidden --check mode, against an isolated catalogue.
addEventListener('DOMContentLoaded', async () => {
  const wait = async predicate => {
    for (let i = 0; i < 100; i++) {
      if (predicate()) return;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    throw new Error('Desktop UI did not become ready');
  };
  try {
    await wait(() => document.querySelector('#content .empty'));
    if (sessionStorage.getItem('omoi-desktop-check') === 'reload') {
      if (document.body.dataset.skin !== 'oak') throw new Error('Appearance did not survive reload');
      sessionStorage.removeItem('omoi-desktop-check');
      window.ipc.postMessage('ready');
      return;
    }
    document.querySelector('[data-page="settings"]').click();
    await wait(() => document.querySelectorAll('[data-theme]').length >= 5);
    const choices = document.querySelectorAll('button[data-theme]');
    if (choices.length !== 5) throw new Error('Missing theme');
    document.querySelector('button[data-theme="oak"]').click();
    sessionStorage.setItem('omoi-desktop-check', 'reload');
    location.reload();
  } catch (error) {
    window.ipc.postMessage(String(error));
  }
});
