use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{W}"),
        library::look_at_top_four_may_take_creature_with_power_at_most::<2>,
        "{2}{W}: Look at the top four cards of your library. You may reveal a creature card with power 2 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::SQUAD_RALLIER
};
