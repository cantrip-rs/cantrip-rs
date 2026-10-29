use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// Exile another target permanent. Return that card to the battlefield under its owner's control at the beginning of
/// the next end step (CR 603.7): a delayed ability about the card exiled, only if it is in exile.
fn exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::delayed::exile_target_then_return_at_next_end_step(g, item, 0, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile, "When Flickerwisp enters, exile another target permanent. Return that card to the battlefield under its owner's control at the beginning of the next end step.")
        .targets(&[targets::OTHER_PERMANENT])],
    delayed: &[delayed_trigger(cantrip_mechanics::delayed::return_subject_to_battlefield, "Return the card exiled with Flickerwisp to the battlefield under its owner's control.")],
    ..oracle::FLICKERWISP
};
