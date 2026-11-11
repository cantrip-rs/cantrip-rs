# Foundation for blocked early-set cards

Base: upstream `6814db9c8c564e8c51c6f698c1f487189baf48f7`.
Branch: `engine/early-set-foundation-20261112`.

Keep each rules feature in a focused commit, with its native tests. Card recipes
and generator adoption follow the shared foundation rather than duplicating it
on separate card branches. Preserve the bounded quantity contract: power,
toughness, counters and life remain signed 32-bit values, damage and X unsigned
32-bit values. Do not restart the paused card campaign as part of this work.

The initial priority list comes from MTGish's November 11 coverage receipt.
Counts below are affected missing cards / cards with no other recorded blocker;
they are not cards already implemented or independently verified end to end.

| Feature | Affected / otherwise clear | Status |
| --- | ---: | --- |
| Blocking as though landwalk were absent | 9 / 9 | Native tests passed (13 landwalk tests) |
| Set only power or only toughness | 4 / 4 | Native tests passed (17 base-P/T tests) |
| World supertype and world rule | 12 / 8 | Queued |
| Bands with other, including grants and removal | 8 / 8 | Queued |
| Optional no combat damage assignment | 8 / 8 | Queued |
| Other players activating abilities | 8 / 5 | Queued |
| Step-bound effect and permission lifetimes | 6 / 5 | Queued |
| Computed limits on announced X | 5 / 3 | Queued |
| Persistent public visibility | 5 / 2 | Queued |

## Blocking exceptions for landwalk

`CardDef::landwalk_exceptions` contains callbacks returning the landwalks ignored
for a particular blocker/attacker pair (CR 509.1b, 609.4, 702.14). Great Wall and
Deadfall name a single landwalk; Staff of the Ages names every landwalk. Callbacks
receive the source's `Asker`, retaining component links and controller context,
and read type words through the source's current text. A named basic landwalk
reference includes its snow counterpart, consistently with other keyword
references.

The engine subtracts the exception only from its local landwalk check. It does
not remove a keyword, change the attacker's characteristics or bypass flying,
protection, another landwalk, or an independent blocking restriction. Exceptions
come from live battlefield sources with functioning abilities, through the same
source list used for ordinary blocking rules. Copies, ability loss, phasing and
control changes use that existing authority. There is no global card-table guard
and no per-pair battlefield scan just to discover exception sources.

Tests cover named and snow-qualified exceptions, another landwalk continuing to
restrict blocking, flying and ordinary blocking restrictions, copied sources,
text changes, ability loss, phasing, controller-dependent scope, and an actual
declaration offering the newly legal block.


## Independent base power and toughness

`EffectKind::SetPower(i32)` and `EffectKind::SetToughness(i32)` are layer-7b
settings (CR 613.4b). Island of Wak-Wak and Singing Tree set power; Sentinel and
Wall of Tombstones set toughness. The other axis is never copied at resolution:
a changing CDA, a later copy, another layer-7b setting, counters, bonuses and
switching power/toughness continue to apply in their own layers and timestamp
order. The existing signed 32-bit bounds and journalled effect storage are
unchanged. Tests include negative and extreme values, changed underlying P/T,
counters, a power/toughness switch, effect ordering and rollback/loop identity.
