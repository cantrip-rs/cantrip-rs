use cantrip_api::*;
use cantrip_mechanics::crew::{becomes_a_creature, crew};
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// Vigilance and trample come from the card's keywords. "Exile an artifact or creature card from your graveyard: This Vehicle
/// becomes an artifact creature until end of turn." (CR 301.7b, 118.3, 400.7) The cost is the exile of one such card from the
/// activating player's own graveyard; the effect is crew's.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            exile_from_graveyard_of(1, filters::artifact_or_creature),
            becomes_a_creature,
            "Exile an artifact or creature card from your graveyard: This Vehicle becomes an artifact creature until end of turn.",
        ),
        crew(3, "Crew 3"),
    ],
    ..oracle::EARTHRUMBLER
};
