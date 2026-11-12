use cantrip_api::*;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_mechanics::statics::{have_vigilance, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    statics: &[
        static_ability(Layer::Ability, itself_at_max_speed, have_vigilance, "Max speed \u{2014} This creature gets +0/+1 and has vigilance."),
        static_ability(Layer::PtModify, itself_at_max_speed, plus_pt::<0, 1>, "Max speed \u{2014} This creature gets +0/+1 and has vigilance.").continues(),
    ],
    ..oracle::SWIFTWING_ASSAILANT
};
