use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The Role under the controller, attached to the mode's target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(|g, item| effects::discard_then_draw(g, item, 1), "Discard a card, then draw a card."),
    mode(tokens::create_a_treasure, "Create a Treasure token."),
    mode(
        create_role,
        "Create a Wicked Role token attached to target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one \u{2014} Discard a card, then draw a card; or create a Treasure token; or create a Wicked Role token attached to target creature you control.",
    )],
    ..oracle::CHARMING_SCOUNDREL
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE, &tokens::WICKED_ROLE];
