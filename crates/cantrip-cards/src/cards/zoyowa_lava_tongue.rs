use cantrip_api::*;
use cantrip_mechanics::targets::filters::any;
use cantrip_mechanics::{history, players, triggers};
use cantrip_oracle as oracle;

/// "Each opponent may discard a card or sacrifice a permanent of their choice. Zoyowa deals 3 damage to each opponent who didn't"
/// (CR 608.2c, 120.3): each opponent in turn order chooses, every sacrifice and discard happens together
/// (`players::each_opponent_may_sacrifice_or_discard`), and then, as the next sentence, Zoyowa deals the damage to those who gave
/// neither, all at once.
fn punish_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let neither = players::each_opponent_may_sacrifice_or_discard(g, item, &CARD, any)?;
    let batch: Vec<(ObjectId, Target, u32)> = neither.into_iter().map(|p| (item.object, Target::Player(p), 3)).collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        punish_each_opponent,
        "At the beginning of your end step, if you descended this turn, each opponent may discard a card or sacrifice a permanent of their choice. Zoyowa deals 3 damage to each opponent who didn't.",
    )
    .condition(history::descended_this_turn)],
    labels: &["Sacrifice a permanent", "Discard a card", "Take 3 damage"],
    ..oracle::ZOYOWA_LAVA_TONGUE
};
