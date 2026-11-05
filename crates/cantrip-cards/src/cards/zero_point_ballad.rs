use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Destroy all creatures with toughness X or less. You lose X life. If X is 6 or more, return a creature card put into a graveyard
/// this way to the battlefield under your control." (CR 608.2c, 701.8a, 107.3a): the creatures are those with toughness X or less
/// as the spell resolves, destroyed together; then you lose X life; then, if the X chosen is 6 or more (the number chosen, not the
/// mana spent), you choose one of the creature cards that went to a graveyard this way, a token being no card (CR 111.1, 111.7) and a card
/// a replacement effect sent elsewhere not being in a graveyard, and it enters under your control.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = item.x;
    let creatures: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.chars(id).toughness.is_some_and(|toughness| i32::from(toughness) <= i32::from(x)))
        .collect();
    let destroyed = g.destroy_all(&creatures);
    g.lose_life(you, i64::from(x));
    if x < 6 {
        return Ok(());
    }
    let cards: Vec<ObjectId> = destroyed.into_iter().filter(|&id| !g.obj(id).token && g.obj(id).zone.is(ZoneKind::Graveyard)).collect();
    if let Some(&card) = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, cards, select::creature_cards(select::exactly(1)))?.first() {
        g.put_onto_battlefield(card, you, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::ZERO_POINT_BALLAD };
