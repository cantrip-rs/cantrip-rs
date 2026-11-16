use cantrip_api::*;
use cantrip_mechanics::statics::{have_double_strike, itself_while_you_committed_a_crime};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        itself_while_you_committed_a_crime,
        have_double_strike,
        "This creature has double strike as long as you've committed a crime this turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )],
    ..oracle::OMENPORT_VIGILANTE
};
