use cantrip_api::*;
use cantrip_mechanics::amass::amass_goblins;
use cantrip_mechanics::{flashback, tokens};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];

/// "Amass Goblins 1. If this spell was cast from a graveyard, amass Goblins 3 instead." The zone it was cast from is the
/// cast record's (CR 601.2a), by flashback or any other permission, and a copy of the spell was never cast (CR 707.10);
/// "instead" replaces the one amass with the other, so exactly one of them happens (CR 701.47a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.history().cast_of(item.object).is_some_and(|cast| cast.from == ZoneKind::Graveyard) {
        amass_goblins::<3>(g, item)
    } else {
        amass_goblins::<1>(g, item)
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{3}{R}"), "Flashback {3}{R}")],
    ..oracle::TIDINGS_OF_WAR
};
