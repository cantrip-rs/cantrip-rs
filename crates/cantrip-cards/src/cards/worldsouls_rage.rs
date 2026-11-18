use cantrip_api::*;
use cantrip_mechanics::{choose, damage_target, select, targets};
use cantrip_oracle as oracle;

/// "Worldsoul's Rage deals X damage to any target. Put up to X land cards from your hand and/or graveyard onto the battlefield
/// tapped" (CR 107.3, 608.2c, 110.5b): the damage first, then the lands - only X is read from the spell on the stack, and the cards
/// chosen are the land cards in the caster's hand and graveyard as the second sentence starts, put onto the battlefield together
/// (CR 603.6a). If the target is illegal as the spell resolves it does not resolve at all (CR 608.2b), so no land comes either.
fn rage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    damage_target(g, item, 0, u32::from(item.x));
    let mut cards: Vec<ObjectId> = g.state().zone(Zone::hand(you)).to_vec();
    cards.extend(g.state().zone(Zone::graveyard(you)).iter().copied());
    let chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, cards, select::land_cards(select::up_to(u32::from(item.x))))?;
    let entering: Vec<(ObjectId, PlayerId, bool)> = chosen.into_iter().map(|card| (card, you, true)).collect();
    g.put_all_onto_battlefield_as(&entering)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ANY], rage), ..oracle::WORLDSOULS_RAGE };
