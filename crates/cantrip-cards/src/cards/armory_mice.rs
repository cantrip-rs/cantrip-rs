use cantrip_api::*;
use cantrip_mechanics::{history, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, history::this_while_you_celebrate, statics::plus_pt::<0, 2>, "Celebration — This creature gets +0/+2 as long as two or more nonland permanents entered the battlefield under your control this turn.")],
    ..oracle::ARMORY_MICE
};
