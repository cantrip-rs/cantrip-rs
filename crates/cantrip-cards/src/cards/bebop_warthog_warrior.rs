use cantrip_api::*;
use cantrip_mechanics::{lands, statics, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2}")],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_of_subtype::<{ lands::sub(Subtype::Rhino) }>,
        statics::have_menace,
        "Rhinos you control have menace.",
    )],
    ..oracle::BEBOP_WARTHOG_WARRIOR
};
