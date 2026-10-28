use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Another target permanent": any permanent on the battlefield but the source itself.
fn another_permanent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if id != asker.source) && (targets::PERMANENT.filter)(q, asker, t)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, delayed::exile_target_and_return_at_next_end_step, "When Flickerwisp enters, exile another target permanent. Return that card to the battlefield under its owner's control at the beginning of the next end step.")
        .targets(&[target(another_permanent)])],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return the card exiled with Flickerwisp to the battlefield under its owner's control.")],
    ..oracle::FLICKERWISP
};
