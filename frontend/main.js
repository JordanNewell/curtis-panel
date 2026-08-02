async function refreshPanel() {
  const snapshot = await window.__TAURI__.core.invoke('get_panel');
  renderPanel(snapshot.panel, snapshot.states);
}

async function onButtonClick(buttonId) {
  try {
    const result = await window.__TAURI__.core.invoke('trigger_action', { buttonId });
    if (!result.success) {
      console.error('Action failed:', result.stderr);
    }
    await refreshPanel();
  } catch (e) {
    console.error('Trigger error:', e);
  }
}

window.onButtonClick = onButtonClick;

refreshPanel().catch(console.error);
