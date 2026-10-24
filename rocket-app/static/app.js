import { selectSets, matchesCard } from '/browse.mjs';

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
  const setList = document.getElementById('set-list');
  if (!dialog || !setList) return;
  const storageKey = `setbinder-browse-v2:${location.pathname}`;
  let saved = {};
  try { saved = JSON.parse(sessionStorage.getItem(storageKey) || '{}') || {}; } catch (_) {}
  if (typeof saved !== 'object' || Array.isArray(saved)) saved = {};
  let cardStates = saved.cards;
  if (!cardStates || typeof cardStates !== 'object' || Array.isArray(cardStates)) cardStates = {};
  const setState = {
    query: typeof saved.sets?.query === 'string' ? saved.sets.query.slice(0, 200) : '',
    filter: ['all', 'incomplete', 'complete', 'unavailable'].includes(saved.sets?.filter) ? saved.sets.filter : 'all',
    sort: ['completion', 'missing', 'name'].includes(saved.sets?.sort) ? saved.sets.sort : 'completion',
  };
  const persist = () => {
    try { sessionStorage.setItem(storageKey, JSON.stringify({ sets: setState, cards: cardStates })); } catch (_) {}
  };
  const setSearch = document.getElementById('set-search');
  const setSort = document.getElementById('set-sort');
  const rows = Array.from(setList.querySelectorAll('.set-row')).map((element) => ({
    element, name: element.dataset.setName, code: element.dataset.setCode,
    available: element.dataset.available === 'true', complete: element.dataset.complete === 'true',
    completion: Number(element.dataset.completion), missing: Number(element.dataset.missing),
  }));
  const pressed = (scope, attribute, value) => {
    scope.querySelectorAll(`[${attribute}]`).forEach((button) => button.setAttribute('aria-pressed', String(button.getAttribute(attribute) === value)));
  };
  const updateSets = () => {
    const visible = selectSets(rows, setState);
    const visibleElements = new Set(visible.map((row) => row.element));
    rows.forEach(({ element }) => { element.hidden = !visibleElements.has(element); });
    // Reorder summary rows without recreating controls or losing focus.
    visible.forEach(({ element }) => setList.append(element));
    pressed(document, 'data-set-filter', setState.filter);
    document.getElementById('set-results').textContent = `${visible.length} of ${rows.length} sets`;
    document.getElementById('set-empty').hidden = visible.length > 0 || rows.length === 0;
    persist();
  };
  setSearch.value = setState.query;
  setSort.value = setState.sort;
  setSearch.addEventListener('input', () => { setState.query = setSearch.value; updateSets(); });
  setSort.addEventListener('change', () => { setState.sort = setSort.value; updateSets(); });
  document.querySelectorAll('[data-set-filter]').forEach((button) => button.addEventListener('click', () => {
    setState.filter = button.dataset.setFilter; updateSets();
  }));
  document.getElementById('reset-sets').addEventListener('click', () => {
    Object.assign(setState, { query: '', filter: 'all', sort: 'completion' });
    setSearch.value = ''; setSort.value = 'completion'; updateSets(); setSearch.focus();
  });
  document.querySelector('[data-set-controls]').hidden = false;
  updateSets();

  let opener;
  function setupCards(code) {
    const search = dialog.querySelector('#card-search');
    if (!search) return;
    const stored = cardStates[code];
    const state = {
      rarity: ['all', 'common', 'uncommon', 'rare', 'mythic', 'special', 'bonus', 'unknown'].includes(stored?.rarity) ? stored.rarity : 'all',
      kind: ['all', 'base', 'variant', 'unclassified'].includes(stored?.kind) ? stored.kind : 'all',
      query: typeof stored?.query === 'string' ? stored.query.slice(0, 200) : '',
      filter: ['all', 'missing', 'collected'].includes(stored?.filter) ? stored.filter : 'all',
      view: stored?.view === 'list' ? 'list' : 'grid',
    };
    const list = dialog.querySelector('#card-list');
    const cards = Array.from(list.querySelectorAll('.card-item')).map((element) => ({
      element, rarity: element.dataset.rarity, kind: element.dataset.kind, name: element.dataset.cardName, number: element.dataset.cardNumber, collected: element.dataset.collected === 'true',
    }));
    const body = dialog.querySelector('.modal-body');
    const updateCards = () => {
      let count = 0;
      cards.forEach((card) => {
        const show = matchesCard(card, state);
        card.element.hidden = !show;
        if (show) count++;
      });
      list.classList.toggle('checklist', state.view === 'list');
      pressed(dialog, 'data-card-filter', state.filter);
      pressed(dialog, 'data-card-view', state.view);
      dialog.querySelector('#card-results').textContent = `${count} of ${cards.length} cards`;
      dialog.querySelector('#card-empty').hidden = count > 0;
      body.scrollTop = 0;
      cardStates[code] = state;
      persist();
    };
    const raritySelect = dialog.querySelector('#card-rarity');
    raritySelect.value = state.rarity;
    raritySelect.addEventListener('change', () => { state.rarity = raritySelect.value; updateCards(); });
    const kindSelect = dialog.querySelector('#card-kind');
    kindSelect.value = state.kind;
    kindSelect.addEventListener('change', () => { state.kind = kindSelect.value; updateCards(); });
    search.value = state.query;
    search.addEventListener('input', () => { state.query = search.value; updateCards(); });
    dialog.querySelectorAll('[data-card-filter]').forEach((button) => button.addEventListener('click', () => {
      state.filter = button.dataset.cardFilter; updateCards();
    }));
    dialog.querySelectorAll('[data-card-view]').forEach((button) => button.addEventListener('click', () => {
      state.view = button.dataset.cardView; updateCards();
    }));
    dialog.querySelector('[data-reset-cards]').addEventListener('click', () => {
      state.query = ''; state.filter = 'all'; state.kind = 'all'; state.rarity = 'all'; raritySelect.value = 'all'; search.value = ''; kindSelect.value = 'all'; updateCards(); search.focus();
    });
    updateCards();
  }
  let detailsRequest;
  const loadingShell = (button) => {
    dialog.innerHTML = '<header class="modal-header"><div><p class="eyebrow">SET DETAILS</p><h2 id="dialog-title"></h2></div><button class="icon-button" data-close aria-label="Close set details">✕</button></header><div class="modal-body" aria-busy="true"><p role="status">Loading this set’s cards…</p></div>';
    dialog.querySelector('h2').textContent = button.closest('.set-row').querySelector('h3').textContent;
  };
  async function loadDetails(button) {
    detailsRequest?.abort();
    const request = new AbortController();
    detailsRequest = request;
    loadingShell(button);
    if (!dialog.open) dialog.showModal();
    dialog.querySelector('[data-close]').focus();
    try {
      const response = await fetch(button.dataset.detailsUrl, { signal: request.signal, headers: { Accept: 'text/html' } });
      if (!response.ok) throw new Error('Set details unavailable');
      const html = await response.text();
      // Ignore replies for a dialog that was closed or replaced while loading.
      if (request.signal.aborted || detailsRequest !== request || !dialog.open) return;
      const fragment = document.createElement('template');
      fragment.innerHTML = html;
      dialog.replaceChildren(fragment.content.cloneNode(true));
      setupCards(button.dataset.open);
      dialog.querySelector('[data-close]').focus();
    } catch (error) {
      if (request.signal.aborted || detailsRequest !== request || !dialog.open) return;
      const body = dialog.querySelector('.modal-body');
      body.removeAttribute('aria-busy');
      body.innerHTML = '<div class="empty-state"><h3>This set could not be loaded.</h3><p>Check your connection and try again. The collection may also have changed.</p><button class="button" data-retry-details>Try again</button></div>';
      body.querySelector('[data-retry-details]').addEventListener('click', () => loadDetails(button));
    }
  }
  document.addEventListener('click', (event) => {
    const button = event.target.closest('[data-open]');
    if (!button) return;
    opener = button;
    loadDetails(button);
  });
  dialog.addEventListener('click', (event) => {
    if (event.target.closest('[data-close]')) { dialog.close(); return; }
    if (event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) dialog.close();
  });
  dialog.addEventListener('close', () => { detailsRequest?.abort(); dialog.replaceChildren(); opener?.focus(); });
})();
