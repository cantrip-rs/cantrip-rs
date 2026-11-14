use cantrip_api::*;
use cantrip_mechanics::{choose_from_hand, effects, statics, triggers};
use cantrip_oracle as oracle;

/// "Whenever you gain life, draw that many cards." (CR 119.3, 121.1): the trigger's `data[0]` is the life gained.
fn draw_that_much(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, item.data[0])
}

/// "Whenever you lose life, discard that many cards. Then if you have no cards in hand, you lose the game." (CR 119.3, 701.9,
/// 104.3e): the cards are chosen from the hand (all of it if it holds fewer, CR 701.9b) and discarded together, and the hand is
/// looked at afterwards, an empty hand (before the discard or after it) losing the game.
fn discard_that_many_then_lose_on_an_empty_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let chosen = choose_from_hand(g, item.controller, item.object, item.data[0])?;
    g.discard_together(&chosen);
    g.end_sentence();
    if g.state().zone(Zone::hand(item.controller)).is_empty() {
        g.lose_game(item.controller);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_IF_CAST,
            |g, item| effects::draw(g, item, 5),
            "When Marina Vendrell's Grimoire enters, if you cast it, draw five cards.",
        ),
        triggered(triggers::YOU_GAIN_LIFE, draw_that_much, "Whenever you gain life, draw that many cards."),
        triggered(
            triggers::YOU_LOSE_LIFE,
            discard_that_many_then_lose_on_an_empty_hand,
            "Whenever you lose life, discard that many cards. Then if you have no cards in hand, you lose the game.",
        ),
    ],
    loss_rules: &[statics::you_cant_lose_from_life],
    play_rules: &[PlayRule::MaximumHandSize(None)],
    ..oracle::MARINA_VENDRELLS_GRIMOIRE
};
