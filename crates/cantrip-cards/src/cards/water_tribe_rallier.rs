use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{5}"),
        library::look_at_top_four_may_take_creature_with_power_at_most::<3>,
        "Waterbend {5}: Look at the top four cards of your library. You may reveal a creature card with power 3 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)",
    )
    .waterbend()],
    ..oracle::WATER_TRIBE_RALLIER
};
