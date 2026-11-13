use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Another target creature you own" (CR 115.1, 108.3): a creature on the battlefield whose owner is the ability's controller,
/// whoever controls it, other than this creature.
fn another_creature_you_own(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_you_own_filter(q, asker, t) && t != Target::Object(asker.source)
}

/// The third mode returns the card at the next end step (CR 603.7) with `delayed::return_subject_to_battlefield`, which puts it
/// onto the battlefield under its owner's control: the target is a creature this ability's controller owns, so that is the
/// control "under your control" asks for. A token has ceased to exist (CR 111.7) and does not return.
static MODAL: ModalDef = modes(&[
    mode(effects::scry_two, "Scry 2."),
    mode(effects::gain_three_life, "You gain 3 life."),
    mode(
        delayed::exile_target_then_return_at_next_end_step_0,
        "Exile another target creature you own. Return it to the battlefield under your control at the beginning of the next end step.",
    )
    .targets(&[target(another_creature_you_own)]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one \u{2014}\n\u{2022} Scry 2.\n\u{2022} You gain 3 life.\n\u{2022} Exile another target creature you own. Return it to the battlefield under your control at the beginning of the next end step.",
    )],
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return it to the battlefield under your control at the beginning of the next end step.",
    )],
    ..oracle::CHARMING_PRINCE
};
