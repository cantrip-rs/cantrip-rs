use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, effects, targets};
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{U}")), "Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"),
        ability(cost("{1}"), effects::first_target_gains_flying, "{1}: Target creature gains flying until end of turn.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::ARNJLOTS_ASCENT
};
