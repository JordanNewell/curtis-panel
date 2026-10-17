function renderPanel(panel, states) {
  const container = document.getElementById('panel');
  container.innerHTML = '';
  container.style.gridTemplateColumns = `repeat(${panel.grid.columns}, 1fr)`;
  container.style.gap = `${panel.grid.spacing}px`;

  const stateById = new Map(states.map(s => [s.id, s]));

  for (const button of panel.buttons) {
    const status = stateById.get(button.id) || { state: 'idle', display_value: null };
    const el = createButtonElement(button, status);
    el.style.gridColumn = `${button.slot[0]} / span ${button.span[0]}`;
    el.style.gridRow = `${button.slot[1]} / span ${button.span[1]}`;
    el.addEventListener('click', () => onButtonClick(button.id));
    container.appendChild(el);
  }

  // deck caption — closes the ledger page so leftover height reads as margin
  let caption = document.getElementById('deck-caption');
  if (!caption) {
    caption = document.createElement('footer');
    caption.id = 'deck-caption';
    container.after(caption);
  }
  const n = panel.buttons.length;
  caption.textContent = `${n} ${n === 1 ? 'entry' : 'entries'}`;
}

window.renderPanel = renderPanel;
