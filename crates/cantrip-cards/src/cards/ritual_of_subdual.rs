use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cantrip_mechanics::cumulative_upkeep::Payment::Mana(mana("{2}")),
            "Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    replacements: &[
        replacement(
            ReplacementKind::ManaProduction,
            mana::a_land_is_tapped,
            mana::produce_instead::<{ Mana::C as u8 }>,
            "If a land is tapped for mana, it produces colorless mana instead of any other type.",
        ),
    ],
    ..oracle::RITUAL_OF_SUBDUAL
};
