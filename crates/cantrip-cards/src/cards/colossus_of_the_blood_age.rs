use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// "When this creature enters, it deals 3 damage to each opponent and you gain 3 life": the life is gained whether or not the
/// damage was dealt (CR 608.2c).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 3)
}

/// "When this creature dies, discard any number of cards, then draw that many cards plus one" (CR 701.9, 121.1): the player
/// chooses how many as the ability resolves, none included, the discards happen together, and the draw is one more than the
/// cards actually discarded - a card a replacement kept in the hand is not counted.
fn discard_any_number_then_draw_that_many_plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let hand: Vec<ObjectId> = g.state().zone(Zone::hand(player)).to_vec();
    let chosen = choose::select(g, player, item.object, Purpose::Discard, hand, select::cards(select::ANY_NUMBER))?;
    let discarded = if chosen.is_empty() { Vec::new() } else { g.discard_together(&chosen) };
    g.draw(player, discarded.len() as u32 + 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, drain, "When this creature enters, it deals 3 damage to each opponent and you gain 3 life."),
        triggered(THIS_DIES, discard_any_number_then_draw_that_many_plus_one, "When this creature dies, discard any number of cards, then draw that many cards plus one."),
    ],
    ..oracle::COLOSSUS_OF_THE_BLOOD_AGE
};
