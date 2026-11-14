use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{discard_chosen, effects};
use cantrip_oracle as oracle;

/// "Draw a card. Then discard a card unless you control another creature with power 2 or less" (CR 121.2, 701.9, 608.2c): the
/// condition is read as the discard is reached, after the draw, with the layers' power; "another" is any creature but this one.
fn draw_then_discard_unless_a_small_friend(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    let small_friend = g
        .permanents_of(item.controller)
        .into_iter()
        .any(|id| id != item.object && g.chars(id).is(CardType::Creature) && g.chars(id).power.is_some_and(|power| power <= 2));
    if !small_friend {
        discard_chosen(g, item.controller, item.object, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        draw_then_discard_unless_a_small_friend,
        "When this creature enters, draw a card. Then discard a card unless you control another creature with power 2 or less.",
    )],
    ..oracle::SPLITSKIN_DOLL
};
