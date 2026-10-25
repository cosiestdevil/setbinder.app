import test from 'node:test';
import assert from 'node:assert/strict';
import { selectSets, groupSets, matchesCard, missingCardList, cardViewState, setViewState, readBrowseUrl, browseSearch } from '../static/browse.mjs';

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
test('rarity combines with printing, ownership and search without hiding unknowns by default', () => {
  const cards = [
    { name: 'Hallowed Fountain', number: '265', rarity: 'rare', kind: 'base', collected: false },
    { name: 'Hallowed Fountain // Hallowed Fountain', number: '347', rarity: 'rare', kind: 'variant', collected: true },
    { name: 'A mythic', number: '10', rarity: 'mythic', kind: 'base', collected: false },
    { name: 'Unknown card', number: '11', rarity: 'unknown', kind: 'unclassified', collected: false },
  ];
  const select = (state) => cards.filter(card => matchesCard(card, state)).map(card => card.number);
  assert.deepEqual(select({}), ['265', '347', '10', '11']);
  assert.deepEqual(select({ rarity: 'rare', kind: 'base', filter: 'missing', query: 'fountain' }), ['265']);
  assert.deepEqual(select({ rarity: 'rare', kind: 'variant', filter: 'collected', query: '#347' }), ['347']);
  assert.deepEqual(select({ rarity: 'mythic', kind: 'base', filter: 'missing' }), ['10']);
  assert.deepEqual(select({ rarity: 'unknown' }), ['11']);
  assert.deepEqual(select({ rarity: 'common' }), []);
  assert.deepEqual(select({ rarity: 'all', kind: 'all', filter: 'all', query: '' }), ['265', '347', '10', '11']);
  for (const rarity of ['common', 'uncommon', 'rare', 'mythic', 'special', 'bonus', 'unknown']) {
    assert.equal(matchesCard({ ...cards[0], rarity }, { rarity }), true);
    assert.equal(matchesCard({ ...cards[0], rarity }, { rarity: 'rare' }), rarity === 'rare');
  }
});

test('copy missing exports exact filtered printings once, without owned cards', () => {
  const base = { name: 'Éowyn, Fearless Knight', number: '1a', rarity: 'rare', kind: 'base', collected: false };
  const cards = [base, base, { ...base, number: '2', collected: true },
    { ...base, number: '3', kind: 'variant' }, { ...base, number: '4', rarity: 'mythic' }];
  assert.equal(missingCardList(cards, { rarity: 'rare', kind: 'base' }, 'ltr'), '1 Éowyn, Fearless Knight (LTR) 1a');
  assert.equal(missingCardList(cards, { filter: 'collected' }, 'ltr'), '');
  assert.equal(missingCardList(cards, { query: 'no matches' }, 'ltr'), '');
  assert.equal(missingCardList([], {}, 'ltr'), '');
  assert.equal(missingCardList(cards, { kind: 'variant', query: '#3' }, 'ltr'), '1 Éowyn, Fearless Knight (LTR) 3');
  assert.equal(missingCardList([{ ...base, name: 'Front // Back\nInjected line', number: '10★' }], {}, 'tst'), '1 Front // Back Injected line (TST) 10★');
  assert.equal(missingCardList(cards, {}, 'ltr').split('\n').length, 3);
});

test('shared URLs round-trip all filters and preserve unrelated query parameters', () => {
  const state = {
    sets: { query: 'Lorwyn & Éowyn', filter: 'incomplete', sort: 'missing' },
    setCode: 'ecl',
    cards: { query: 'Front // Back #10★ & +', filter: 'missing', kind: 'base', rarity: 'mythic', view: 'list' },
  };
  const search = browseSearch(state, '?campaign=test&set=old&q=stale');
  assert.deepEqual(readBrowseUrl(search), state);
  assert.equal(new URLSearchParams(search).get('campaign'), 'test');
  assert.equal(new URLSearchParams(search).getAll('set').length, 1);
  assert.equal(browseSearch(readBrowseUrl(search), search), search);
});
test('explicit defaults override saved filters and closing a set clears card URL fields', () => {
  assert.equal(readBrowseUrl('?unrelated=1'), null);
  assert.deepEqual(readBrowseUrl('?browse=1'), { sets: setViewState(), setCode: null, cards: cardViewState() });
  const search = browseSearch({ sets: setViewState(), setCode: null }, '?set=ecl&rarity=rare&q=old&owned=missing');
  assert.equal(search, '?browse=1');
  const opened = readBrowseUrl('?set=ecl');
  assert.equal(opened.setCode, 'ecl');
  assert.deepEqual(opened.cards, cardViewState());
});
test('invalid and oversized URL state is bounded and normalized', () => {
  const state = readBrowseUrl('?set=../../bad&status=bad&sort=bad&owned=bad&printing=bad&rarity=bad&layout=bad&q=' + 'x'.repeat(500));
  assert.equal(state.setCode, null);
  assert.deepEqual(state.sets, setViewState());
  assert.deepEqual(state.cards, { ...cardViewState(), query: 'x'.repeat(200) });
  assert.deepEqual(cardViewState(null), cardViewState());
  assert.deepEqual(setViewState(null), setViewState());
  assert.equal(new URLSearchParams(browseSearch({setCode: 'BAD/SET'})).has('set'), false);
});


test('release search finds sibling sets while filters and sorting remain per set', () => {
  const family = [
    { ...sets[1], code: 'fin', name: 'Final Fantasy', releaseCode: 'fin', releaseName: 'Final Fantasy' },
    { ...sets[3], code: 'fic', name: 'Commander', releaseCode: 'fin', releaseName: 'Final Fantasy' },
    { ...sets[2], code: 'fca', name: 'Through the Ages', releaseCode: 'fin', releaseName: 'Final Fantasy' },
  ];
  assert.deepEqual(selectSets(family, { query: 'final fantasy' }).map(s => s.code), ['fic', 'fin', 'fca']);
  assert.deepEqual(selectSets(family, { query: 'FIN', filter: 'incomplete', sort: 'missing' }).map(s => s.code), ['fca', 'fin']);
  assert.deepEqual(selectSets(family, { query: 'FCA' }).map(s => s.code), ['fca']);
  const grouped = groupSets(selectSets([...family, sets[2], sets[0]]));
  assert.deepEqual(grouped.map(g => g.code), ['fin', 'alt', 'unk']);
  assert.deepEqual(grouped[0].sets.map(s => s.code), ['fic', 'fin', 'fca']);
  assert.deepEqual(groupSets(selectSets(family, { query: 'absent' })), []);
});
