use cantrip_api::*;
use cantrip_mechanics::{flashback, history, tokens};
use cantrip_oracle as oracle;

/// The 3/3 blue Robot Warrior artifact creature token (CR 111.1, 111.4).
static ROBOT_WARRIOR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Robot Warrior)
};

/// "Create a 3/3 blue Robot Warrior artifact creature token. Then if this spell was cast from a graveyard, put two +1/+1 counters
/// on that token": the token and then the counters are each an event, judged as they happen (CR 608.2c), and the question is asked of how the spell was cast
/// (CR 601.2a), so a copy of it, never cast, makes the plain 3/3 (CR 707.10). Each token made gets the counters, as a replacement
/// that makes more has them all be "that token" (CR 614.1a, 111.1).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &ROBOT_WARRIOR, 1)?;
    if history::cast_from_graveyard(g.history(), item.object).is_some() {
        let counters: Vec<(ObjectId, CounterKind, i64)> = made.into_iter().map(|token| (token, CounterKind::PlusOne, 2)).collect();
        g.add_counters_together(&counters);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{5}{U}"), "Flashback {5}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)")],
    ..oracle::RETRIEVE_THE_ESPER
};

pub static EXTRA: &[&CardDef] = &[&ROBOT_WARRIOR];
