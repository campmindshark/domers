const status = document.querySelector('#engine-status');

document.querySelector('#start-engine')?.addEventListener('click', () => {
  status.textContent = 'running';
});

document.querySelector('#stop-engine')?.addEventListener('click', () => {
  status.textContent = 'stopped';
});
