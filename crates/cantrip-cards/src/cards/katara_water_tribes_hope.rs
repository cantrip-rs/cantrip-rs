use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers, your_turn};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_an_ally, "When Katara enters, create a 1/1 white Ally creature token."),
        ability(
            cost("{X}"),
            effects::creatures_you_control_have_base_pt_x,
            "Waterbend {X}: Creatures you control have base power and toughness X/X until end of turn. X can't be 0. Activate only during your turn. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)",
        )
        .waterbend()
        .x_cant_be_zero()
        .condition(your_turn),
    ],
    ..oracle::KATARA_WATER_TRIBES_HOPE
};
