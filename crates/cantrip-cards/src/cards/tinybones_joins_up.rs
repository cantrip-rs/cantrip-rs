use cantrip_api::*;
use cantrip_mechanics::{each_discards_a_card, targets, triggers};
use cantrip_oracle as oracle;

/// "Any number of target players" (CR 115.1d, 115.3): none, or each player once.
const fn any_number_of_players(label: &'static str) -> TargetSpec {
    targets::PLAYER.between(0, TargetSpec::ANY_NUMBER).labelled(label)
}

/// "When Tinybones Joins Up enters, any number of target players each discard a card." Each target still legal as it
/// resolves (CR 608.2b) chooses a card in turn order from the active player and they are discarded together (CR 101.4,
/// 701.9); a player with no card in hand discards nothing.
fn discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let legal = targets::legal_players(g, item);
    let players: Vec<PlayerId> = g.state().apnap().filter(|p| legal.contains(p)).collect();
    each_discards_a_card(g, &players, item.object)?;
    Ok(())
}

/// "Whenever a legendary creature you control enters, any number of target players each mill a card and lose 1 life."
/// All the targeted players still legal mill at once (CR 701.17), and each loses 1 life whether or not a card was milled.
fn mill_and_lose(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let legal = targets::legal_players(g, item);
    let mills: Vec<(PlayerId, u32)> = legal.iter().map(|&p| (p, 1)).collect();
    g.mill_together(&mills);
    let losses: Vec<(PlayerId, i32)> = legal.into_iter().map(|p| (p, 1)).collect();
    g.lose_life_together(&losses);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, discard, "When Tinybones Joins Up enters, any number of target players each discard a card.")
            .targets(&[any_number_of_players("Players who each discard a card")]),
        triggered(
            triggers::LEGENDARY_CREATURE_YOU_CONTROL_ENTERS,
            mill_and_lose,
            "Whenever a legendary creature you control enters, any number of target players each mill a card and lose 1 life.",
        )
        .targets(&[any_number_of_players("Players who each mill a card and lose 1 life")]),
    ],
    ..oracle::TINYBONES_JOINS_UP
};
