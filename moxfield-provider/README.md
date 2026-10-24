# Public Moxfield collections

The adapter reads public collections from Moxfield's internal collection search
endpoint. It uses an identifiable Set Binder user agent, requires no login, and
never writes to Moxfield. This API is private and can change or reject automated
requests; Moxfield's FAQ recommends contacting support about community integrations:
https://moxfield.com/help/help-articles/faq#moxfield-api

Only paper entries with positive quantity and no proxy flag count as owned.
Finishes, languages, conditions and quantities are combined by set code and
collector number, matching Set Binder's exact-printing progress rules. Rarity
and base/variant classification come from the embedded Scryfall database.

All pages must load successfully. Repeated entries, changing pagination totals,
missing printing identities and incomplete pages produce errors instead of
silently showing an incomplete collection. Requests have a 30-second timeout;
collections are limited to 100,000 entries and 100 pages. A collection changing
while loading may need to be retried. A change that preserves all pagination
metadata cannot be detected without an upstream snapshot API.

Run deterministic tests with `cargo test -p moxfield-provider`. To check live
access, set `MOXFIELD_TEST_COLLECTION` to a public collection ID and run:

```sh
cargo test -p moxfield-provider public_collection_smoke -- --ignored
```

Local access does not verify access from Cloud Run. Check a public collection
and one set-details request after deployment; Cloudflare policy can differ by
network. Keep the existing Archidekt path available if Moxfield access fails.
