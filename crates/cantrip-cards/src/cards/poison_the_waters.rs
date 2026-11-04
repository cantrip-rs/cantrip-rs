use cantrip_api::*;
use cantrip_mechanics::effects::pump_all_creatures;
use cantrip_mechanics::hand::discard_chosen_artifact_or_creature_from_revealed_hand;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Mode 1: "All creatures get -1/-1 until end of turn" (CR 611.2c): the creatures on the battlefield as it resolves, whoever
/// controls them; a creature left with toughness 0 or less dies as a state-based action (CR 704.5f).
fn all_creatures_get_minus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_all_creatures(g, item.object, -1, -1);
    Ok(())
}

/// Mode 2: "Target player reveals their hand. You choose an artifact or creature card from it. That player discards that card."
/// Nothing happens if the target is no longer legal (CR 608.2b); the reveal is the looking, so no choice is asked when the hand
/// holds no artifact or creature card.
fn reveal_and_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    discard_chosen_artifact_or_creature_from_revealed_hand(g, item, player)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(all_creatures_get_minus_one, "All creatures get -1/-1 until end of turn."),
    mode(reveal_and_discard, "Target player reveals their hand. You choose an artifact or creature card from it. That player discards that card.")
        .targets(&[targets::PLAYER]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::POISON_THE_WATERS };
