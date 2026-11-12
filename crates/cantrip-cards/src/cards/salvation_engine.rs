use cantrip_api::*;
use cantrip_mechanics::statics::{other_artifact_creatures_you_control, plus_pt};
use cantrip_mechanics::{crew, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, other_artifact_creatures_you_control, plus_pt::<2, 2>, "Other artifact creatures you control get +2/+2.")],
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            effects::return_first_target_to_battlefield,
            "Whenever this Vehicle attacks, return up to one target artifact card from your graveyard to the battlefield.",
        )
        .targets(&[up_to_one(targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD.filter)]),
        crew::crew(6, "Crew 6 (Tap any number of creatures you control with total power 6 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::SALVATION_ENGINE
};
