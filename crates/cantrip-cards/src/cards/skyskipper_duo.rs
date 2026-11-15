use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, exile up to one other target creature you control. Return it to the battlefield under its owner's
/// control at the beginning of the next end step." (CR 603.7, 400.7): the exile and the delayed ability that brings it back are one
/// effect, so it returns as a new object under its owner's control, whoever controls it by then, and a token exiled this way has
/// ceased to exist and stays gone (CR 111.7). The shared slot-0 body.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        delayed::exile_target_then_return_at_next_end_step_0,
        "When this creature enters, exile up to one other target creature you control. Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )
    .targets(&[up_to_one(targets::OTHER_CREATURE_YOU_CONTROL.filter)])],
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )],
    ..oracle::SKYSKIPPER_DUO
};
