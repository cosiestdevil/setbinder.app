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
