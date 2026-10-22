use cantrip_api::*;
use cantrip_mechanics::triggers::LANDFALL;
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

fn is_creature(q: &dyn Query, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Creature)
}

/// "Landfall — Whenever a land you control enters, draw a card. Then if this is the first time this ability has resolved
/// this turn, reveal cards from the top of your library until you reveal a creature card. Put that card onto the
/// battlefield and the rest on the bottom of your library in a random order." Which resolution this is is asked of this
/// turn's history (CR 608.2m). The draw comes first, so a card it draws is not one the reveal looks through (CR 608.2c);
/// a library holding no creature card is revealed whole and put back in a random order (CR 701.20, 401.4).
fn landfall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    if g.history().resolutions(item.object, item.ability) != 0 {
        return Ok(());
    }
    let (found, rest) = library::reveal_until(g, item.controller, 1, &is_creature);
    if let Some(card) = found.first().copied() {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    g.bottom_in_random_order(item.controller, &rest);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        landfall,
        "Landfall \u{2014} Whenever a land you control enters, draw a card. Then if this is the first time this ability has resolved this turn, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::NISSA_LEYLINE_TAMER
};
