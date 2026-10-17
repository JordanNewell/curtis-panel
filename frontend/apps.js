// App system: render the installed-app list, drive the app view (iframe),
// handle the "record unavailable" state with auto-retry.
(function () {
  const invoke = window.__TAURI__.core.invoke;
  const list = document.getElementById('app-list');
  const frame = document.getElementById('app-frame');
  const unavailable = document.getElementById('app-unavailable');
  const unavailableName = document.getElementById('app-unavailable-name');
  const retryBtn = document.getElementById('app-retry');

  let activeApp = null;
  let loadTimer = null;
  let retryDelay = 5000;
  let loaded = true;

  function showUnavailable(name) {
    frame.hidden = true;
    frame.src = 'about:blank';
    unavailable.hidden = false;
    unavailableName.textContent = name;
    scheduleRetry();
  }

  function scheduleRetry() {
    clearTimeout(loadTimer);
    loadTimer = setTimeout(() => { if (activeApp) openApp(activeApp); }, retryDelay);
    retryDelay = Math.min(retryDelay * 2, 60000);
  }

  // Manifests may point at a local file (`entry`, an absolute OS path) or a
  // remote `url`. Normalize backslashed Windows paths to file:// URLs.
  function appTarget(app) {
    if (app.url) return app.url;
    if (app.entry) return new URL('file:///' + app.entry.replace(/\\/g, '/')).href;
    return 'about:blank';
  }

  function openApp(app) {
    if (app !== activeApp) { retryDelay = 5000; }
    activeApp = app;
    unavailable.hidden = true;
    clearTimeout(loadTimer);
    frame.hidden = false;
    loaded = false;
    // Load timeout: if the iframe never fires `load` in 6s, treat as down.
    loadTimer = setTimeout(() => {
      if (!loaded) showUnavailable(app.name);
    }, 6000);
    frame.src = appTarget(app);
  }

  frame.addEventListener('load', () => {
    loaded = true;
    clearTimeout(loadTimer);
    unavailable.hidden = true;
  });

  retryBtn.addEventListener('click', () => { if (activeApp) openApp(activeApp); });

  async function renderApps() {
    const snap = await invoke('get_dock');
    list.innerHTML = '';
    for (const app of snap.apps) {
      const btn = document.createElement('button');
      btn.className = 'app-entry';
      btn.textContent = app.name;
      btn.addEventListener('click', () => openApp(app));
      list.appendChild(btn);
    }
    if (snap.apps.length === 0) {
      const p = document.createElement('p');
      p.className = 'app-empty';
      p.textContent = 'no apps filed. drop a folder with app.curt.yaml into ~/.curtis/panel/apps/';
      list.appendChild(p);
    }
    // Marketplace placeholder (stub — v2 wires the real directory)
    const soon = document.createElement('p');
    soon.className = 'app-soon';
    soon.textContent = 'directory — coming soon';
    list.appendChild(soon);
  }

  window.addEventListener('dock:snapshot', () => renderApps().catch(console.error));
  renderApps().catch(console.error);
})();
