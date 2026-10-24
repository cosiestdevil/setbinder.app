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
    // Moving existing rows preserves their dialogs and avoids duplicating the card data.
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
      kind: ['all', 'base', 'variant', 'unclassified'].includes(stored?.kind) ? stored.kind : 'all',
      query: typeof stored?.query === 'string' ? stored.query.slice(0, 200) : '',
      filter: ['all', 'missing', 'collected'].includes(stored?.filter) ? stored.filter : 'all',
      view: stored?.view === 'list' ? 'list' : 'grid',
    };
    const list = dialog.querySelector('#card-list');
    const cards = Array.from(list.querySelectorAll('.card-item')).map((element) => ({
      element, kind: element.dataset.kind, name: element.dataset.cardName, number: element.dataset.cardNumber, collected: element.dataset.collected === 'true',
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
      state.query = ''; state.filter = 'all'; state.kind = 'all'; search.value = ''; kindSelect.value = 'all'; updateCards(); search.focus();
    });
    updateCards();
  }
  document.addEventListener('click', (event) => {
    const button = event.target.closest('[data-open]');
    if (!button) return;
    const template = document.getElementById(button.dataset.open);
    if (!template) return;
    opener = button;
    dialog.replaceChildren(template.content.cloneNode(true));
    dialog.querySelector('[data-close]').addEventListener('click', () => dialog.close());
    setupCards(template.dataset.setCode);
    dialog.showModal();
  });
  dialog.addEventListener('click', (event) => {
    if (event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) dialog.close();
  });
  dialog.addEventListener('close', () => { dialog.replaceChildren(); opener?.focus(); });
})();
