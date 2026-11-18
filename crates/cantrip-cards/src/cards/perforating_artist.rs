use cantrip_api::*;
use cantrip_mechanics::players;
use cantrip_mechanics::raid;
use cantrip_mechanics::targets::filters::nonland;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "Each opponent loses 3 life unless that player sacrifices a nonland permanent of their choice or discards a card"
/// (`players::each_opponent_loses_life_unless_sacrifices_or_discards`, once).
fn punish_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    players::each_opponent_loses_life_unless_sacrifices_or_discards(g, item, &CARD, nonland, 1, 3)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOUR_END_STEP, punish_each_opponent, "Raid \u{2014} At the beginning of your end step, if you attacked this turn, each opponent loses 3 life unless that player sacrifices a nonland permanent of their choice or discards a card.").condition(raid::you_attacked_this_turn)],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Lose 3 life"],
    ..oracle::PERFORATING_ARTIST
};
