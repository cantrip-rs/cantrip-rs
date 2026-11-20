use cantrip_api::*;
use cantrip_mechanics::effects::{plus_one_counter_on_this, target_creature_gets_plus_x_x_for_this_power};
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::{self, THIS_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::token_you_control_enters),
            plus_one_counter_on_this,
            "Whenever a token you control enters, put a +1/+1 counter on this creature.",
        ),
        triggered(
            THIS_ATTACKS,
            target_creature_gets_plus_x_x_for_this_power,
            "Whenever this creature attacks, another target attacking creature gets +X/+X until end of turn, where X is this creature's power.",
        )
        .targets(&[targets::ANOTHER_ATTACKING_CREATURE]),
    ],
    ..oracle::WILDWOOD_MENTOR
};
