import test from 'node:test';
import assert from 'node:assert/strict';
import { selectSets, matchesCard } from '../static/browse.mjs';

const sets = [
  { name: 'Unknown', code: 'unk', available: false, complete: false, missing: 0, completion: 0 },
  { name: 'Ikoria', code: 'iko', available: true, complete: false, missing: 126, completion: .67 },
  { name: 'Almost there', code: 'alt', available: true, complete: false, missing: 1, completion: .5 },
  { name: 'All done', code: 'all', available: true, complete: true, missing: 0, completion: 1 },
];
test('unknown sets never appear complete or almost finished', () => {
  assert.deepEqual(selectSets(sets, { filter: 'complete' }).map(s => s.code), ['all']);
  assert.deepEqual(selectSets(sets, { filter: 'incomplete', sort: 'missing' }).map(s => s.code), ['alt', 'iko']);
  assert.equal(selectSets(sets, { sort: 'missing' }).at(-1).code, 'unk');
  assert.deepEqual(selectSets(sets, { filter: 'unavailable' }).map(s => s.code), ['unk']);
});
test('search and status combine; clearing the search restores the requested order', () => {
  assert.equal(selectSets(sets, { query: 'IKO', filter: 'incomplete' })[0].name, 'Ikoria');
  assert.equal(selectSets(sets, { query: 'IKO', filter: 'complete' }).length, 0);
  assert.deepEqual(selectSets(sets, { sort: 'completion' }).map(s => s.code), ['all', 'iko', 'alt', 'unk']);
  assert.equal(selectSets(sets, { sort: 'name' })[0].code, 'all');
  assert.equal(sets[0].code, 'unk', 'sorting must not mutate the source collection');
});
test('collector numbers match exactly and ownership is respected', () => {
  const card = { name: 'Éowyn, Fearless Knight', number: '1a', collected: true };
  assert.equal(matchesCard(card, { query: '#1a' }), true);
  assert.equal(matchesCard(card, { query: '1' }), false);
  assert.equal(matchesCard(card, { query: 'eowyn knight', filter: 'collected' }), true);
  assert.equal(matchesCard(card, { query: 'eowyn', filter: 'missing' }), false);
  assert.equal(matchesCard({ ...card, number: '10' }, { query: '1' }), false);
});
test('printing classification combines with ownership and collector search', () => {
  const base = { name: 'Card', number: '2', collected: false, kind: 'base' };
  const variant = { ...base, number: '10a', collected: true, kind: 'variant' };
  const unknown = { ...base, number: '3', kind: 'unclassified' };
  assert.equal(matchesCard(base, { kind: 'base', filter: 'missing' }), true);
  assert.equal(matchesCard(variant, { kind: 'base', filter: 'collected' }), false);
  assert.equal(matchesCard(variant, { kind: 'variant', filter: 'collected', query: '#10a' }), true);
  assert.equal(matchesCard(unknown, { kind: 'unclassified' }), true);
  assert.equal(matchesCard(unknown, { kind: 'variant' }), false);
});
