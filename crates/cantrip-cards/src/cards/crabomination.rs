use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::library::random_of;
use cantrip_oracle as oracle;

/// "A card at random from a zone": the card at a random place in it, nothing from an empty zone.
fn at_random(g: &mut dyn Game, zone: Zone) -> Option<ObjectId> {
    let cards: Vec<ObjectId> = g.state().zone(zone).to_vec();
    random_of(g, &cards)
}

/// Target opponent exiles the top card of their library, a card at random from their graveyard, and a card at random
/// from their hand. You may cast a spell from among cards exiled this way without paying its mana cost. The three
/// cards leave together; a zone with nothing in it contributes nothing; only a card that could be cast at all (not a
/// land, a target to be found for it) is a choice.
fn plunder(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(opponent) = item.target(0) else { return Ok(()) };
    let mut taken: Vec<ObjectId> = Vec::new();
    taken.extend(g.state().zone(Zone::library(opponent)).first().copied());
    taken.extend(at_random(g, Zone::graveyard(opponent)));
    taken.extend(at_random(g, Zone::hand(opponent)));
    if taken.is_empty() {
        return Ok(());
    }
    // Only the cards that really arrived in exile: a replacement can send a card somewhere else instead.
    let exiled = g.exile_all(&taken);
    let exiled = cantrip_mechanics::library::still_exiled(g, &exiled);
    let castable: Vec<ObjectId> = exiled.iter().copied().filter(|&c| g.could_cast_during_resolution(c)).collect();
    if let Some(&card) = choose::cards_from(g, item.controller, item.object, Purpose::CastFree, exiled, castable, 1, true)?.first() {
        g.cast_during_resolution(card, Some(ManaCost::ZERO))?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[way(Cost { mana: mana("{5}{B}{B}"), permanents: Some(PermanentCost { count: 1, filter: cantrip_mechanics::targets::filters::artifact, action: PermanentAction::Sacrifice, tapped: false, excluded: None, together: None }), emerge: true, ..Cost::NONE }, ZoneKind::Hand, "Emerge from artifact {5}{B}{B}")],
    abilities: &[triggered(THIS_ENTERS, plunder, "When this creature enters, target opponent exiles the top card of their library, a card at random from their graveyard, and a card at random from their hand. You may cast a spell from among cards exiled this way without paying its mana cost.")
        .targets(&[targets::OPPONENT])],
    ..oracle::CRABOMINATION
};
