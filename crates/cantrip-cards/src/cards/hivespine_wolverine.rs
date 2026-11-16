use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This creature fights target creature token" (CR 701.14a-b): the source and the token each deal damage equal to their power to the
/// other, at once. Nothing fights if the token is no longer a legal target, or this creature is no longer a creature on the battlefield.
fn this_fights_target_creature_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(token) = targets::object(g, item, 0) else { return Ok(()) };
    g.fight(item.object, token);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::plus_one_counter_on_target, "Put a +1/+1 counter on target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(this_fights_target_creature_token, "This creature fights target creature token.").targets(&[targets::CREATURE_TOKEN]),
    mode(effects::destroy_first_target, "Destroy target artifact or enchantment.").targets(&[targets::ARTIFACT_OR_ENCHANTMENT]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one \u{2014}\n\u{2022} Put a +1/+1 counter on target creature you control.\n\u{2022} This creature fights target creature token.\n\u{2022} Destroy target artifact or enchantment.",
    )],
    ..oracle::HIVESPINE_WOLVERINE
};
