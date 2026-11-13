use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::ManaAndLife(mana("{B}"), 1),
            "Cumulative upkeep—Pay {B} and 1 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    replacements: &[
        replacement(
            ReplacementKind::ManaProduction,
            mana::a_land_is_tapped,
            mana::produce_instead::<{ Mana::B as u8 }>,
            "If a land is tapped for mana, it produces {B} instead of any other type.",
        ),
    ],
    ..oracle::INFERNAL_DARKNESS
};
