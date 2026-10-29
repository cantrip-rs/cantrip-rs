//! The creatures that crewed a Vehicle (CR 702.122a) are in the history: an `Event::Crewed` for each creature tapped to pay
//! the cost of its crew ability, read with `History::crewed` for the turn. A creature tapped for another cost that taps
//! creatures crewed nothing; one that has left the battlefield still counts; crewing twice names a creature twice. Test-only cards.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A Vehicle with "Crew 1" as the core reads it, and a Drum with the same tapping cost that is no crew.
static VAN: CardDef = CardDef {
    name: "Crewed Test Van",
    cost: ManaCost { generic: 3, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]),
    power: Some(4),
    toughness: Some(4),
    abilities: &[ability(
        Cost { tap_creatures: Some(TapCreatures { power: 1, greater_with: Some(Keyword::CrewsAsThoughPowerTwoGreater) }), ..Cost::NONE },
        nothing,
        "Crew 1",
    )],
    ..CardDef::BLANK
};
static DRUM: CardDef = CardDef {
    name: "Crewed Test Drum",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { tap_creatures: Some(TapCreatures { power: 1, greater_with: None }), ..Cost::NONE }, nothing, "Tap a creature.")],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Crewed Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef { name: "Crewed Test Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

/// A game at your first main phase with the Van, the Drum and three Bears on the battlefield.
fn game() -> (Game, ObjectId, ObjectId, [ObjectId; 3]) {
    let table = Box::leak(Box::new(CardTable::new([&VAN, &DRUM, &BEAR, &LAND])));
    let filler = table.id_of(&LAND);
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 30], vec![filler; 30]], ..Default::default() });
    let van = put(&mut game, &VAN, Zone::BATTLEFIELD);
    let drum = put(&mut game, &DRUM, Zone::BATTLEFIELD);
    let bears = [put(&mut game, &BEAR, Zone::BATTLEFIELD), put(&mut game, &BEAR, Zone::BATTLEFIELD), put(&mut game, &BEAR, Zone::BATTLEFIELD)];
    for _ in 0..150 {
        game.run();
        let r = game.pending().cloned().expect("a question");
        if r.player == YOU && game.state.turn.active == YOU && game.state.turn.step() == Step::PrecombatMain && matches!(r.kind, RequestKind::Priority { .. }) {
            return (game, van, drum, bears);
        }
        let answer = match &r.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected {other:?}"),
        };
        game.answer(r.id, r.player, answer).unwrap();
    }
    panic!("main phase never reached");
}

/// Activates `source`'s ability, tapping `creatures` for its cost, and lets it resolve.
fn activate(game: &mut Game, source: ObjectId, creatures: &[ObjectId]) {
    let r = game.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("unexpected {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source)).expect("offered").clone();
    game.answer(r.id, r.player, Answer::Act(Action::of(&option))).unwrap();
    game.run();
    let r = game.pending().cloned().expect("a question");
    assert!(matches!(r.kind, RequestKind::TapForPower { .. }), "unexpected {:?}", r.kind);
    game.answer(r.id, r.player, Answer::Cards(creatures.to_vec())).unwrap();
    game.run();
    while !game.state.stack.is_empty() {
        let r = game.pending().cloned().expect("a question");
        game.answer(r.id, r.player, Answer::Pass).unwrap();
        game.run();
    }
}

fn crewed(game: &Game, vehicle: ObjectId) -> Vec<ObjectId> {
    game.history().crewed(vehicle).collect()
}

/// CR 702.122a: each of the creatures tapped to crew crewed the Vehicle, in the order chosen.
#[test]
fn every_creature_tapped_to_crew_crewed_the_vehicle() {
    let (mut game, van, _, [a, b, c]) = game();
    assert_eq!(crewed(&game, van), []);
    activate(&mut game, van, &[b, a]);
    assert_eq!(crewed(&game, van), [b, a]);
    assert!(!game.state.obj(c).tapped, "an untapped creature crewed nothing");
}

/// Crewing again names its creatures after the first crew's, and one that has left the battlefield still counts.
#[test]
fn crewing_twice_names_both_crews_and_a_creature_that_died_still_counts() {
    let (mut game, van, _, [a, b, c]) = game();
    activate(&mut game, van, &[a]);
    activate(&mut game, van, &[c]);
    cantrip_api::Game::destroy(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }, a);
    assert!(game.state.obj(a).became.is_some(), "the Bear died");
    assert_eq!(crewed(&game, van), [a, c]);
    assert!(!crewed(&game, van).contains(&b));
}

/// A cost that taps creatures and is no crew crews nothing, and another Vehicle's crew is its own.
#[test]
fn a_creature_tapped_for_another_cost_crewed_nothing() {
    let (mut game, van, drum, [a, ..]) = game();
    activate(&mut game, drum, &[a]);
    assert!(game.state.obj(a).tapped);
    assert_eq!(crewed(&game, van), []);
    assert_eq!(crewed(&game, drum), []);
}
