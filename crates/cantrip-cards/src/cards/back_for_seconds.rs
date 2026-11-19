use cantrip_api::*;
use cantrip_mechanics::{bargain, choose, select, targets};
use cantrip_oracle as oracle;

/// "Return up to two target creature cards from your graveyard to your hand. If this spell was bargained, you may put one of
/// those cards with mana value 4 or less onto the battlefield instead of putting it into your hand." (CR 608.2b, 702.166b): only
/// the targets still legal as it resolves are returned, together; a bargained spell first lets its controller choose one of them
/// with mana value 4 or less (a graveyard card's mana value is that of its front face), which goes onto the battlefield while the
/// others still go to the hand, all of them in one move (CR 101.4, 608.2c: one instruction).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut back = targets::legal_objects(g, item);
    let mut battlefield = None;
    if bargain::bargained(item) {
        let eligible: Vec<ObjectId> = back.iter().copied().filter(|&card| g.mana_value(card) <= 4).collect();
        battlefield = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, eligible, select::cards(select::up_to(1)))?.first().copied();
        back.retain(|&card| Some(card) != battlefield);
    }
    let onto: Vec<(ObjectId, PlayerId, bool, PlayerId)> = battlefield.map(|card| (card, you, false, you)).into_iter().collect();
    g.put_onto_battlefield_and_into_hand(&onto, &back, MoveReason::Other, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], effect),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::BACK_FOR_SECONDS
};
