use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, put a +1/+1 counter on target creature and you gain 1 life." The ability has one target, judged
/// again as it resolves: if it is illegal then the ability does not resolve and neither instruction happens (CR 608.2b, the
/// 2024-07-26 ruling). It may target itself, or any creature, an opponent's included (CR 115.1d).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::plus_one_counter_on_target_and_you_gain_1_life,
        "When this creature enters, put a +1/+1 counter on target creature and you gain 1 life.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::SUNSHOWER_DRUID
};
