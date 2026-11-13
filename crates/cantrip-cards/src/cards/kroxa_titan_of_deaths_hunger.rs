use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_mechanics::{each_discards_a_card, escape, hand};
use cantrip_oracle as oracle;

/// "Each opponent discards a card, then each opponent who didn't discard a nonland card this way loses 3 life." Each
/// opponent chooses in turn order and the cards are discarded together (CR 101.4, 701.9); an opponent with no card in
/// hand, one a land card was discarded from, and one whose card a replacement put into a hidden zone unrevealed (CR
/// 701.9c) didn't discard a nonland card, and loses 3 life. The discard is done before the life is lost (CR 608.2c).
fn each_opponent_discards_then_loses_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != you).collect();
    let discarded = each_discards_a_card(g, &opponents, item.object)?;
    g.end_sentence();
    let losses: Vec<(PlayerId, i32)> = opponents
        .into_iter()
        .filter(|&who| !discarded.iter().any(|&card| g.obj(card).owner == who && hand::discarded_nonland_card(&*g, card)))
        .map(|who| (who, 3))
        .collect();
    g.lose_life_together(&losses);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        escape::sacrifice_unless_escaped("When Kroxa enters, sacrifice it unless it escaped."),
        triggered(
            triggers::THIS_ENTERS_OR_ATTACKS,
            each_opponent_discards_then_loses_life,
            "Whenever Kroxa enters or attacks, each opponent discards a card, then each opponent who didn't discard a nonland card this way loses 3 life.",
        ),
    ],
    ways: &[escape::way(mana("{B}{B}{R}{R}"), 5, "Escape—{B}{B}{R}{R}, Exile five other cards from your graveyard")],
    ..oracle::KROXA_TITAN_OF_DEATHS_HUNGER
};
