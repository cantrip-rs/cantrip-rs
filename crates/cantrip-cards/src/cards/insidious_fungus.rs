use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target artifact" / "Destroy target enchantment" (CR 701.8): nothing if the target is no longer legal (CR 608.2b); an
/// indestructible one stays (the verb's).
fn destroy_its_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(destroy_its_target, "Destroy target artifact.").targets(&[targets::ARTIFACT]),
    mode(destroy_its_target, "Destroy target enchantment.").targets(&[targets::ENCHANTMENT]),
    mode(effects::draw_a_card_then_may_put_a_land_tapped, "Draw a card. Then you may put a land card from your hand onto the battlefield tapped."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_ability(
        cost("{2}").and(SACRIFICE_THIS),
        &MODAL,
        "{2}, Sacrifice this creature: Choose one \u{2014} destroy target artifact; destroy target enchantment; or draw a card, then you may put a land card from your hand onto the battlefield tapped.",
    )],
    ..oracle::INSIDIOUS_FUNGUS
};
