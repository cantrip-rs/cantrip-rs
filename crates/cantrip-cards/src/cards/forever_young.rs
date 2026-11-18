use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Any number of target creature cards from your graveyard".
fn creature_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
}

/// Put any number of target creature cards from your graveyard on top of your library, in an order you choose (asked
/// only when there is more than one card, since one card has no order), then draw a card.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let cards = targets::legal_objects(g, item);
    if !cards.is_empty() {
        let mut order = cards.clone();
        if cards.len() > 1 {
            let ask = RequestKind::DistributeCards {
                source: item.object,
                cards: cards.clone(),
                destinations: vec![Destination { kind: DestinationKind::LibraryTop, min: 0, max: u32::MAX, eligible: cards.clone() }],
                may_shuffle: false,
            };
            if let Answer::Placements { placements, .. } = g.ask(player, ask)?
                && placements.first().is_some_and(|p| p.len() == cards.len())
            {
                order = placements[0].clone();
            }
        }
        g.put_into_library(&order, LibraryPlace::Top);
    }
    g.draw(player, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[up_to(TargetSpec::ANY_NUMBER, creature_card_in_your_graveyard)], effect),
    ..oracle::FOREVER_YOUNG
};
