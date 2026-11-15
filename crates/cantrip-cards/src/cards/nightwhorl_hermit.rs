use cantrip_api::*;
use cantrip_mechanics::statics::{have_cant_be_blocked, itself_with_threshold, plus_pt};
use cantrip_oracle as oracle;

/// "Threshold - As long as there are seven or more cards in your graveyard, this creature gets +1/+0 and can't be blocked"
/// (CR 207.2c, 604.1, 509.1b): two static effects, +1/+0 in layer 7c and the keyword in layer 6, each asked again whenever
/// characteristics are computed. Once it has been blocked, reaching seven doesn't unblock it (CR 509.1).
pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            itself_with_threshold,
            plus_pt::<1, 0>,
            "Threshold \u{2014} As long as there are seven or more cards in your graveyard, this creature gets +1/+0 and can't be blocked.",
        ),
        static_ability(
            Layer::Ability,
            itself_with_threshold,
            have_cant_be_blocked,
            "Threshold \u{2014} As long as there are seven or more cards in your graveyard, this creature gets +1/+0 and can't be blocked.",
        ),
    ],
    ..oracle::NIGHTWHORL_HERMIT
};
