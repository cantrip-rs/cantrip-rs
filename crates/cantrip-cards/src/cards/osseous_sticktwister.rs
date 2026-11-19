use cantrip_api::*;
use cantrip_mechanics::targets::filters::nonland;
use cantrip_mechanics::{delirium, fight::damage_equal_to_power, players, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// An intervening "if" (CR 603.4): delirium.
fn has_delirium(q: &dyn Query, asker: &Asker) -> bool {
    delirium(q, asker.controller)
}

/// "Each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal
/// to its power to each opponent who didn't sacrifice a permanent or discard a card this way."
///
/// Each opponent in turn order chooses, and every sacrifice and every discard happens together
/// (`players::each_opponent_may_sacrifice_or_discard`). The damage comes after, from the creature's power as it last existed if it
/// has left (CR 608.2h).
fn each_opponent_may_sacrifice_or_discard_else_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let neither = players::each_opponent_may_sacrifice_or_discard(g, item, &CARD, nonland)?;
    let power = damage_equal_to_power(g, item.object);
    let batch: Vec<(ObjectId, Target, u32)> = neither.into_iter().map(|p| (item.object, Target::Player(p), power)).collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        each_opponent_may_sacrifice_or_discard_else_damage,
        "Delirium \u{2014} At the beginning of your end step, if there are four or more card types among cards in your graveyard, each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
    )
    .condition(has_delirium)],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Do neither"],
    ..oracle::OSSEOUS_STICKTWISTER
};
