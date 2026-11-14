use cantrip_api::*;
use cantrip_mechanics::{counter_target_spell_or_ability, manifest_dread, targets, triggers};
use cantrip_oracle as oracle;

/// "Counter target spell. Its controller manifests dread." (CR 701.6, 701.62a, 608.2b): the spell is countered only while it is still a
/// legal target, and its controller, read while it is still on the stack, then manifests dread. When the spell is gone the whole
/// ability does nothing (CR 608.2b), so nobody manifests.
fn counter_target_spell_then_its_controller_manifests_dread(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(controller) = counter_target_spell_or_ability(g, item, 0) else { return Ok(()) };
    g.end_sentence();
    manifest_dread::manifest_dread(g, controller, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        counter_target_spell_then_its_controller_manifests_dread,
        "When this creature enters, counter target spell. Its controller manifests dread.",
    )
    .targets(&[targets::SPELL])],
    ..oracle::FEAR_OF_IMPOSTORS
};
