use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    // "{T}, Pay 1 life: Add one mana of any color.", the mana ability the other Caves are given: the Cave is its source (CR 113.6).
    delayed: &[mana_ability(TAP.and(life(1)), add_any_color(1), "{T}, Pay 1 life: Add one mana of any color.")],
    // "Other Caves you control have ..." (CR 613.1f, layer 6): not Forgotten Monument itself, and not an opponent's Cave.
    statics: &[static_ability(
        Layer::Ability,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Cave) }>,
        statics::grant_delayed_ability,
        "Other Caves you control have \"{T}, Pay 1 life: Add one mana of any color.\"",
    )],
    ..oracle::FORGOTTEN_MONUMENT
};
