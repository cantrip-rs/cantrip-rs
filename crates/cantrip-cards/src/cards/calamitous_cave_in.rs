use cantrip_api::*;
use cantrip_mechanics::effects::damage_each_creature_and_planeswalker;
use cantrip_mechanics::{permanents_with_subtype_you_control, subtype_cards_in_graveyard};
use cantrip_oracle as oracle;

/// "Calamitous Cave-In deals X damage to each creature and each planeswalker, where X is the number of Caves you control plus the
/// number of Cave cards in your graveyard" (CR 608.2, 120.3, 205.3i). X is counted once as the spell resolves; the damage is dealt
/// all at once. "Cave" is a land type word of its text, read through its words (CR 612.2).
fn damage_per_cave(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cave = g.words(item.object).subtype(Subtype::Cave);
    let you = item.controller;
    let x = permanents_with_subtype_you_control(g, you, cave) + subtype_cards_in_graveyard(g, you, cave);
    damage_each_creature_and_planeswalker(g, item, x as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], damage_per_cave), ..oracle::CALAMITOUS_CAVE_IN };
