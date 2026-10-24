// Shared by the browser and dependency-free Node regression tests.
export const normalize = (value) => String(value).normalize('NFD').replace(/\p{M}/gu, '').toLowerCase().trim();

export function selectSets(sets, { query = '', filter = 'all', sort = 'completion' } = {}) {
  const terms = normalize(query).split(/\s+/).filter(Boolean);
  const matches = sets.filter((set) => {
    const text = normalize(`${set.name} ${set.code}`);
    const status = filter === 'all' || (filter === 'unavailable' && !set.available)
      || (filter === 'complete' && set.complete) || (filter === 'incomplete' && set.available && !set.complete);
    return status && terms.every((term) => text.includes(term));
  });
  const byName = (a, b) => a.name.localeCompare(b.name) || a.code.localeCompare(b.code);
  return matches.sort((a, b) => {
    if (sort === 'name') return byName(a, b);
    if (a.available !== b.available) return a.available ? -1 : 1;
    return (sort === 'missing' ? a.missing - b.missing : b.completion - a.completion) || byName(a, b);
  });
}

export function matchesCard(card, { query = '', filter = 'all', kind = 'all', rarity = 'all' } = {}) {
  if (rarity !== 'all' && card.rarity !== rarity) return false;
  if (kind !== 'all' && card.kind !== kind) return false;
  if (filter === 'missing' && card.collected || filter === 'collected' && !card.collected) return false;
  const search = normalize(query);
  if (!search) return true;
  // A collector-number search should find #1, rather than also #10 and #100.
  if (search.startsWith('#') || /^\d+[a-z★]*$/i.test(search)) return normalize(card.number) === search.replace(/^#/, '');
  return search.split(/\s+/).every((term) => normalize(card.name).includes(term));
}

// One copy of each exact missing printing; ownership quantities do not multiply rows.
export function missingCardList(cards, state, setCode) {
  const seen = new Set();
  return cards.filter(card => {
    if (card.collected || !matchesCard(card, state) || seen.has(card.number)) return false;
    seen.add(card.number);
    return true;
  }).map(card => `1 ${String(card.name).replace(/\s+/g, ' ').trim()} (${setCode.toUpperCase()}) ${card.number}`).join('\n');
}

const oneOf = (value, choices, fallback) => choices.includes(value) ? value : fallback;
const queryText = value => typeof value === 'string' ? value.slice(0, 200) : '';
export function cardViewState(value = {}) {
  return {
    query: queryText(value?.query),
    filter: oneOf(value?.filter, ['all', 'missing', 'collected'], 'all'),
    kind: oneOf(value?.kind, ['all', 'base', 'variant', 'unclassified'], 'all'),
    rarity: oneOf(value?.rarity, ['all', 'common', 'uncommon', 'rare', 'mythic', 'special', 'bonus', 'unknown'], 'all'),
    view: oneOf(value?.view, ['grid', 'list'], 'grid'),
  };
}
export function setViewState(value = {}) {
  return {
    query: queryText(value?.query),
    filter: oneOf(value?.filter, ['all', 'incomplete', 'complete', 'unavailable'], 'all'),
    sort: oneOf(value?.sort, ['completion', 'missing', 'name'], 'completion'),
  };
}
const browseKeys = ['browse', 'set', 'sets', 'status', 'sort', 'q', 'owned', 'printing', 'rarity', 'layout'];
export function readBrowseUrl(search) {
  const params = new URLSearchParams(search);
  if (!browseKeys.some(key => params.has(key))) return null;
  const code = params.get('set') || '';
  return {
    setCode: /^[a-z0-9]{1,16}$/.test(code) ? code : null,
    sets: setViewState({ query: params.get('sets'), filter: params.get('status'), sort: params.get('sort') }),
    cards: cardViewState({ query: params.get('q'), filter: params.get('owned'), kind: params.get('printing'), rarity: params.get('rarity'), view: params.get('layout') }),
  };
}
export function browseSearch({ sets, setCode, cards }, search = '') {
  const params = new URLSearchParams(search);
  browseKeys.forEach(key => params.delete(key));
  // This marker makes omitted defaults explicit, overriding a recipient's saved filters.
  params.set('browse', '1');
  const setState = setViewState(sets);
  const setDefaults = setViewState();
  for (const [field, key] of [['query', 'sets'], ['filter', 'status'], ['sort', 'sort']]) {
    if (setState[field] !== setDefaults[field]) params.set(key, setState[field]);
  }
  if (/^[a-z0-9]{1,16}$/.test(setCode || '')) {
    params.set('set', setCode);
    const state = cardViewState(cards);
    const defaults = cardViewState();
    for (const [field, key] of [['query', 'q'], ['filter', 'owned'], ['kind', 'printing'], ['rarity', 'rarity'], ['view', 'layout']]) {
      if (state[field] !== defaults[field]) params.set(key, state[field]);
    }
  }
  return `?${params}`;
}
