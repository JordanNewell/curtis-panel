// Dock chrome: tabs, theme, slide-in/out coordination, size presets.
(function () {
  const invoke = window.__TAURI__.core.invoke;
  const listen = window.__TAURI__.event.listen;

  // --- theme toggle (paper default, persisted) ---
  const themeBtn = document.getElementById('theme-toggle');
  function applyTheme(theme) {
    document.body.classList.toggle('theme-ink', theme === 'ink');
    document.body.classList.toggle('theme-paper', theme !== 'ink');
    themeBtn.textContent = theme === 'ink' ? 'paper' : 'ink';
    localStorage.setItem('curtis-panel-theme', theme);
  }
  applyTheme(localStorage.getItem('curtis-panel-theme') || 'paper');
  themeBtn.addEventListener('click', () =>
    applyTheme(document.body.classList.contains('theme-ink') ? 'paper' : 'ink'));

  // --- view tabs ---
  document.querySelectorAll('#dock-tabs .tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('#dock-tabs .tab').forEach(t => {
        t.classList.remove('active');
        t.setAttribute('aria-selected', 'false');
      });
      document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
      tab.classList.add('active');
      tab.setAttribute('aria-selected', 'true');
      document.getElementById(`view-${tab.dataset.view}`).classList.add('active');
    });
  });

  // --- size presets (keyboard: 1/2/3 while panel shown) ---
  window.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    const sizes = { '1': 'ledger', '2': 'folio', '3': 'full' };
    if (sizes[e.key]) invoke('set_dock_size', { size: sizes[e.key] });
  });

  // --- slide coordination with the Rust shell ---
  const dock = document.getElementById('dock');

  async function initEdgeAndMotion() {
    const snap = await invoke('get_dock');
    dock.classList.remove('edge-right', 'edge-left', 'edge-top', 'edge-bottom');
    dock.classList.add(`edge-${snap.edge}`, `motion-${snap.motion}`);
    if (snap.shown) dock.classList.add('open');
    window.dispatchEvent(new CustomEvent('dock:snapshot', { detail: snap }));
  }

  listen('dock://visibility', (event) => {
    const shown = event.payload;
    if (shown) {
      dock.classList.add('open');
    } else {
      dock.classList.remove('open'); // Rust hides the window after 240ms
    }
  });

  initEdgeAndMotion().catch(console.error);
})();
