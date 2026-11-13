use cantrip_api::*;
use cantrip_mechanics::statics::{itself_with_threshold, plus_pt};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, effects::you_mill::<3>, "When this creature enters, mill three cards.")],
    statics: &[static_ability(
        Layer::PtModify,
        itself_with_threshold,
        plus_pt::<2, 1>,
        "Threshold \u{2014} This creature gets +2/+1 as long as there are seven or more cards in your graveyard.",
    )],
    ..oracle::BILLOWING_SHRIEKMASS
};
