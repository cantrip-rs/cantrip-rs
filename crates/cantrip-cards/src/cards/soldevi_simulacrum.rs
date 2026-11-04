use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, effects};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cantrip_mechanics::cumulative_upkeep::Payment::Mana(ManaCost { snow: 0, generic : 1, x
            : 0, symbols : [0, 0, 0, 0, 0, 0], hybrid : [None, None, None, None, None,
            None], twobrid : [0, 0, 0, 0, 0], phyrexian : [0, 0, 0, 0, 0] }),
            "Cumulative upkeep"
        ),
        ability(
            cost("{1}"),
            effects::this_gets::<1, 0>,
            "{1}: This creature gets +1/+0 until end of turn.",
        ),
    ],
    ..oracle::SOLDEVI_SIMULACRUM
};
