# Early-set foundation refresh

The 37-commit early-set foundation was rebased from its previous upstream base
onto `27fffc942f71e8c64854bb68da096a67b9498578`. The original foundation head is
`b5000fa915e3470c80b2063f29d42daa9c3a522a`.

Integration keeps upstream's additional-phase schedule, simultaneous untaps,
CDA/component text exclusions, triggered mana-resolution procedure, and target
announcement batching. Named activation payers use upstream's explicit-player
exertion procedure and bitset. The foundation's World bookkeeping is initialized
in the added hypothetical mana-production query as in other query copies.
Bands-with-other definitions take the same non-mana branch as other static
abilities in upstream's new mana-definition accessor.

Skipped beginning steps also apply in additional beginning phases. An additional
draw step during the starting player's first turn is distinct from the skipped
ordinary draw step. Regression cases cover those interactions. The delayed
combat test now creates its additional combat through the phase API.

Counter IDs retain upstream's Vow at 91; Cube, Doom and Glyph append at 92, 93
and 94. Pin (95) and Sleight (96), needed by Voodoo Doll and Chromatic Armor,
are now real shared prerequisites rather than a test-only counter overlay.
They use the existing dense representation, adding two `i32` counter entries
per object. Existing quantity domains are retained. The new production generator
snapshot must be generated from this final API, including its capability markers
and vocabulary, before reserving generator-ready cards.

The original worktree and its research files were preserved while the rebase was
performed in an isolated checkout. Build caches are private reflinks. Native
optimized builds use `RUST_MIN_STACK=16777216`, as earlier foundation builds did,
because the compiler's LLVM optimizer over the full card table can exhaust its
default stack. This is a compiler build setting.

Validation receipts are recorded after the exact rebased source passes the native
workspace, managed fast suite, generated-card checks and matched benchmark.
