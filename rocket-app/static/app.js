(() => {
  'use strict';
  const theme = document.getElementById('theme');
  const savedTheme = () => { try { return localStorage.getItem('setbinder-theme') || 'system'; } catch (_) { return 'system'; } };
  theme.value = ['light', 'dark'].includes(savedTheme()) ? savedTheme() : 'system';
  theme.addEventListener('change', () => {
    if (theme.value === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme.value;
    try { localStorage.setItem('setbinder-theme', theme.value); } catch (_) {}
  });

  const form = document.querySelector('[data-loading-form]');
  const resetForm = () => {
    if (!form) return;
    form.removeAttribute('aria-busy');
    form.querySelector('button').disabled = false;
    form.querySelector('[data-submit-label]').textContent = 'View collection';
    form.querySelector('.loading-message').hidden = true;
  };
  form?.addEventListener('submit', () => {
    form.setAttribute('aria-busy', 'true');
    form.querySelector('button').disabled = true;
    form.querySelector('[data-submit-label]').textContent = 'Opening your binder…';
    form.querySelector('.loading-message').hidden = false;
  });
  window.addEventListener('pageshow', resetForm);

  const dialog = document.getElementById('set-dialog');
  if (!dialog) return;
  let opener;
  document.addEventListener('click', (event) => {
    const button = event.target.closest('[data-open]');
    if (!button) return;
    const template = document.getElementById(button.dataset.open);
    if (!template) return;
    opener = button;
    dialog.replaceChildren(template.content.cloneNode(true));
    dialog.querySelector('[data-close]').addEventListener('click', () => dialog.close());
    dialog.showModal();
  });
  dialog.addEventListener('click', (event) => {
    if (event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) dialog.close();
  });
  dialog.addEventListener('close', () => { dialog.replaceChildren(); opener?.focus(); });
})();
