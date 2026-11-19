use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Exile target nontoken creature. Return it to the battlefield under its owner's control at the
/// beginning of the next end step." (CR 603.7, 400.7): a delayed ability about the card exiled, so a creature that is not
/// in exile at the end step (it was exiled somewhere else, or it is a new object) does not return.
pub static SWIFT_SPIRAL: CardDef = CardDef {
    spell: spell(&[targets::NONTOKEN_CREATURE], delayed::exile_target_then_return_at_next_end_step_0),
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )],
    ..oracle::SWIFT_SPIRAL
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)")],
    adventure: Some(&SWIFT_SPIRAL),
    ..oracle::TWINING_TWINS
};

pub static EXTRA: &[&CardDef] = &[&SWIFT_SPIRAL];
