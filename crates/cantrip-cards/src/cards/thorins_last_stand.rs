use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target artifact or enchantment. You gain 2 life." The life is a second instruction (CR 608.2c), gained though the
/// permanent is indestructible or regenerates.
fn destroy_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    g.end_sentence();
    g.gain_life(item.controller, 2);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::creatures_you_control_get::<2, 1>, "Creatures you control get +2/+1 until end of turn."),
    mode(destroy_and_gain_life, "Destroy target artifact or enchantment. You gain 2 life.").targets(&[targets::ARTIFACT_OR_ENCHANTMENT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::THORINS_LAST_STAND };
