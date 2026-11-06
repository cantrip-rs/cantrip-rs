# Bound objects and open creature restrictions

CR 700.4 defines dying as moving from the battlefield to a graveyard. A replacement
about a previously identified permanent therefore continues to cover that same
object after it loses its creature type. It still does not cover a return to hand,
another permanent, or a later battlefield incarnation (CR 400.7, 614.1a).

`EXILE_INSTEAD_OF_DYING` applies that bound-identity predicate. Its thirteen current
callers include nine "that creature" instructions, Essence Burn's "that permanent",
Disintegrate's creature condition evaluated on resolution, and Chandra/Torch the
Tower's actual damaged permanents. Their setup determines which object is covered;
the replacement does not add a new current-type restriction.

Anger of the Gods instead says "a creature dealt damage this way". Its separate
`CREATURE_EXILED_INSTEAD_OF_DYING` additionally checks the current Creature type
when that permanent would die. Red Sun's Zenith and Yamabushi's Flame print this
same open creature restriction. Anger retains all actual damaged permanent
identities, including redirected damage to a planeswalker that could become a
creature later; it does not filter that receipt at the time damage is dealt.

The original Disintegrate type-loss scenario fails against the old helper. The
old Chandra/Lion Sash scenario likewise encoded a false graveyard expectation;
its corrected exile expectation fails against the old implementation and passes
with the bound predicate. Five focused native controls distinguish the two
predicates, hand returns, other objects and new incarnations. Restoring the old
blanket Creature-or-Planeswalker check fails two controls; restoring the fix
passes all five. All thirteen existing caller scenario filters pass: 59 cases,
zero failures or missing cards.

This interpretation follows current Oracle wording and the current rules. The
caller audit does not claim a separate official ruling specifically covering
type loss. Evidence is retained under the card-lane integration directory in
`replacement-damage-lineage-proof/disintegrate-helper-review.md` and
`bound-exile-helper-*` artifacts.
