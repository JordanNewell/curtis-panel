function createButtonElement(button, status) {
  const el = document.createElement('button');
  el.className = `button shape-${button.shape} state-${status.state}`;
  el.dataset.buttonId = button.id;

  const label = document.createElement('span');
  label.className = 'button-label';
  label.textContent = button.label || button.id;
  el.appendChild(label);

  if (status.display_value) {
    const value = document.createElement('span');
    value.className = 'button-value';
    value.textContent = status.display_value;
    el.appendChild(value);
  }

  return el;
}

window.createButtonElement = createButtonElement;
