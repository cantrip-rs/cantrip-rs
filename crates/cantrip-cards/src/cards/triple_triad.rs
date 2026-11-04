use cantrip_api::*;
use cantrip_mechanics::{top_of_library, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, each player exiles the top card of their library. Until end of turn, you may play the card
/// you own exiled this way and each other card exiled this way with lesser mana value than it without paying their mana costs."
/// (CR 406.2, 101.4, 118.9, 305.1): the cards leave together, one from each library that has a card; the controller may play the one
/// card of their own, and each other card whose mana value (CR 202.3; X is 0 in exile) is less than its, strictly, a land among
/// them as a land play (ruling 2025-06-06). A card that did not reach exile, or an empty library, is not among them; with no card
/// of the controller's exiled there is no card to compare with and nothing may be played.
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let tops: Vec<ObjectId> = players.iter().flat_map(|&p| top_of_library(g, p, 1)).collect();
    let exiled: Vec<ObjectId> = g.exile_all(&tops).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    let Some(&mine) = exiled.iter().find(|&&c| g.obj(c).owner == item.controller) else { return Ok(()) };
    let limit = g.mana_value(mine);
    let grant = PlayGrant { instead: Some(ManaCost::ZERO), lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    for card in exiled {
        if card == mine || g.mana_value(card) < limit {
            g.allow_play(card, item.controller, grant);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        exile_and_play,
        "At the beginning of your upkeep, each player exiles the top card of their library. Until end of turn, you may play the card you own exiled this way and each other card exiled this way with lesser mana value than it without paying their mana costs.",
    )],
    ..oracle::TRIPLE_TRIAD
};
