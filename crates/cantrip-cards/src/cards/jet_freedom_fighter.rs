use cantrip_api::*;
use cantrip_mechanics::{
    effects, targets,
    triggers::{THIS_DIES, THIS_ENTERS},
};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::damage_first_target_by_creatures_you_control, "When Jet enters, he deals damage equal to the number of creatures you control to target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(THIS_DIES, effects::plus_one_counter_on_each_target, "When Jet dies, put a +1/+1 counter on each of up to two target creatures.")
            .targets(&[targets::CREATURE.between(0, 2)]),
    ],
    ..oracle::JET_FREEDOM_FIGHTER
};
