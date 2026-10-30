use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, triggers};
use cantrip_oracle as oracle;

/// Ward (CR 702.21) and an exile-until-leaves triggered ability (CR 610.3, 603.6a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2}"),
        triggered(triggers::THIS_ENTERS, exile_target_until_leaves_effect,
            "When this artifact enters, exile target nonland permanent an opponent controls until this artifact leaves the battlefield.")
            .targets(&[targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::MALFUNCTIONING_HOLODECK
};
