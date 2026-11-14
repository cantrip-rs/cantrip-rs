use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        library::look_for_a_creature_with_power_at_most::<5, 2>,
        "When this creature dies, look at the top five cards of your library. You may reveal a creature card with power 2 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::LIVING_PHONE
};
