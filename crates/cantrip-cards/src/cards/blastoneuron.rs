use cantrip_api::*;
use cantrip_mechanics::{copy, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Exile target creature card ... Create a token that's a copy of it. That token gains haste. Sacrifice it at the beginning
/// of the next end step." Shared with Chrome Dome (`copy::create_token_copy_with_haste_sacrificed_at_end_step`). An
/// illegal target does nothing (CR 608.2b).
fn exile_copy_with_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(card);
    copy::create_token_copy_with_haste_sacrificed_at_end_step(g, item, exiled, 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, exile_copy_with_haste, "When this creature enters, exile target creature card with mana value 3 or less from your graveyard. Create a token that's a copy of it. That token gains haste. Sacrifice it at the beginning of the next end step.")
            .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD]),
        triggered(
            triggers::CARDS_LEAVE_YOUR_GRAVEYARD,
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever one or more cards leave your graveyard, each opponent loses 1 life and you gain 1 life.",
        )
        .once_per_batch(),
    ],
    delayed: &[copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::BLASTONEURON
};
