function renderPanel(panel, states) {
  const container = document.getElementById('panel');
  container.innerHTML = '';
  container.style.gridTemplateColumns = `repeat(${panel.grid.columns}, 1fr)`;
  container.style.gridTemplateRows = `repeat(${panel.grid.rows}, 1fr)`;
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
}

window.renderPanel = renderPanel;
