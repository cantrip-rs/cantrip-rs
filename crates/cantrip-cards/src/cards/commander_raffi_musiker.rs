use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Exile up to one target card from a graveyard. You gain 2 life." (CR 701.13, 119.3): the life is gained even when no
/// target was chosen; if a chosen target is illegal, the whole ability does not resolve (CR 608.2b).
fn exile_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::exile_target(g, item, 0);
    g.gain_life(item.controller, 2);
    Ok(())
}

/// "Investigate. You lose 2 life." (CR 701.16a, 119.3): create the Clue, then lose life.
fn investigate_and_lose_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)?;
    g.lose_life(item.controller, 2);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(exile_and_gain_life, "Exile up to one target card from a graveyard. You gain 2 life.")
        .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()]),
    mode(investigate_and_lose_life, "Investigate. You lose 2 life."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODES,
        "When Commander Musiker enters, choose one — Exile up to one target card from a graveyard. You gain 2 life.; Investigate. You lose 2 life.",
    )],
    ..oracle::COMMANDER_RAFFI_MUSIKER
};
