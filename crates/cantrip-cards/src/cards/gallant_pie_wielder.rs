use cantrip_api::*;
use cantrip_mechanics::{history, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        history::this_while_you_celebrate,
        statics::have_double_strike,
        "Celebration — This creature has double strike as long as two or more nonland permanents entered the battlefield under your control this turn.",
    )],
    ..oracle::GALLANT_PIE_WIELDER
};
