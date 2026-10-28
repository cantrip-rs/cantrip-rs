use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(Payment::Mana(mana("{U}{R}")), "Cumulative upkeep {U}{R} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)")],
    ..oracle::FIRESTORM_HELLKITE
};
