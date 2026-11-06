use cantrip_api::*;
use cantrip_mechanics::{targets, top_of_library, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// "Two target players each reveal the top card of their library. They each lose life equal to the mana value of the card revealed
/// by the other player. Then they each put the card they revealed into their hand." The players still legal as it resolves
/// (CR 608.2b) reveal, an empty library revealing nothing (CR 701.20); each player's life loss is the mana value of the other's
/// card as it was revealed (CR 608.2h), nothing when the other revealed none; then the revealed cards move (a move, not a draw).
fn reveal_and_trade(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players = targets::legal_players(g, item);
    let mut revealed: Vec<Option<(ObjectId, u32)>> = Vec::new();
    for &player in &players {
        let top = top_of_library(g, player, 1).first().copied();
        revealed.push(top.map(|card| {
            let value = g.mana_value(card);
            g.reveal(player, &[card]);
            (card, value)
        }));
    }
    g.end_sentence();
    instruction(g, |g| {
        for (i, &player) in players.iter().enumerate() {
            if let Some((_, value)) = revealed.get(1 - i).copied().flatten() {
                g.lose_life(player,(value) as i32);
            }
        }
    });
    g.end_sentence();
    let cards: Vec<ObjectId> = revealed.into_iter().flatten().map(|(card, _)| card).collect();
    g.return_to_hand_revealed(&cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        reveal_and_trade,
        "At the beginning of your end step, two target players each reveal the top card of their library. They each lose life equal to the mana value of the card revealed by the other player. Then they each put the card they revealed into their hand.",
    )
    .targets(&[targets::PLAYER.between(2, 2)])],
    ..oracle::PARKER_LUCK
};
