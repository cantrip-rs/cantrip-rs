use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{CREATURE_AN_OPPONENT_CONTROLS_DIES, THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::creatures_your_opponents_control_get::<-2, -2>,
            "When this creature enters, creatures your opponents control get -2/-2 until end of turn.",
        ),
        triggered(
            CREATURE_AN_OPPONENT_CONTROLS_DIES,
            effects::that_player_loses_life::<2>,
            "Whenever a creature an opponent controls dies, that player loses 2 life.",
        ),
    ],
    ..oracle::MASSACRE_WURM
};
