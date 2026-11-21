//! A discover (CR 701.57) is announced as `Event::Discovered` once it has been carried out (CR 701.57a), naming the player and the
//! discover value, the library running out first included (CR 701.57b: the player has still discovered). Test-only cards: a creature
//! whose ability is "discover 2", a spell with mana value 2 (the discover's limit) and one with mana value 3 (over it).

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn discovers_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::discover::discover(g, item.controller, 2)?;
    Ok(())
}

static DISCOVERER: CardDef = CardDef {
    name: "Discover Event Discoverer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(Cost::NONE, discovers_two, "Discover 2.")],
    ..CardDef::BLANK
};
static CHEAP: CardDef = CardDef { name: "Discover Event Cheap Spell", types: TypeSet::of(&[CardType::Sorcery]), cost: mana("{2}"), ..CardDef::BLANK };
static DEAR: CardDef = CardDef { name: "Discover Event Dear Spell", types: TypeSet::of(&[CardType::Sorcery]), cost: mana("{3}"), ..CardDef::BLANK };
static LAND: CardDef = CardDef { name: "Discover Event Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A game at your first main phase with an empty library and the discoverer in play, then `library` put into it (first = top).
fn game(library: &[&'static CardDef]) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&DISCOVERER, &CHEAP, &DEAR, &LAND])));
    let cheap = table.id_of(&CHEAP);
    let mut g = Game::new(table, &Setup { decks: vec![vec![cheap; 20], vec![cheap; 20]], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    while let Some(&card) = g.state.zone(Zone::library(YOU)).first() {
        g.state.zone_remove(Zone::library(YOU), card);
    }
    for def in library.iter().rev() {
        let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::library(YOU)));
        g.state.zone_insert(Zone::library(YOU), 0, id);
    }
    let mut obj = Obj::new(g.cards.id_of(&DISCOVERER), YOU, Zone::BATTLEFIELD);
    obj.timestamp = g.state.timestamp();
    let id = g.state.push_obj(obj);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    (g, id)
}

/// Activates the discoverer and plays the stack out, declining to cast what is found; the events recorded meanwhile.
fn discover(g: &mut Game, source: ObjectId) -> Vec<Event> {
    let start = g.log.len();
    g.run();
    let r = g.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("unexpected {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source)).expect("the ability is offered");
    g.answer(r.id, r.player, Answer::Act(Action::of(option))).unwrap();
    for _ in 0..20 {
        g.run();
        let r = g.pending().cloned().expect("a question");
        let answer = match &r.kind {
            RequestKind::Priority { .. } if g.state.stack.is_empty() => break,
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::May { .. } => Answer::No,
            other => panic!("unexpected {other:?}"),
        };
        g.answer(r.id, r.player, answer).unwrap();
    }
    g.log[start..].iter().map(|r| r.event).collect()
}

fn discovered(events: &[Event]) -> Vec<(PlayerId, u32)> {
    events.iter().filter_map(|e| if let Event::Discovered { player, n } = *e { Some((player, n)) } else { None }).collect()
}

fn in_hand(g: &Game, def: &'static CardDef) -> bool {
    g.state.zone(Zone::hand(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(def))
}

#[test]
fn discovering_is_announced_with_its_value_once_the_card_found_is_in_hand() {
    let (mut g, source) = game(&[&LAND, &DEAR, &CHEAP]);
    let events = discover(&mut g, source);
    // The land and the mana value 3 spell are passed over (exiled, then put on the bottom); the mana value 2 one is found and, not cast, goes to hand.
    assert_eq!(discovered(&events), vec![(YOU, 2)], "{events:?}");
    assert!(in_hand(&g, &CHEAP), "the card found is in the hand");
    assert!(!in_hand(&g, &DEAR) && !in_hand(&g, &LAND));
}

#[test]
fn discovering_with_a_library_that_runs_out_is_still_announced() {
    let (mut g, source) = game(&[&LAND, &DEAR]);
    let events = discover(&mut g, source);
    // CR 701.57b: no card with mana value 2 or less was found, and the player has still discovered.
    assert_eq!(discovered(&events), vec![(YOU, 2)], "{events:?}");
    assert!(!in_hand(&g, &LAND) && !in_hand(&g, &DEAR), "nothing is put into the hand");
}

#[test]
fn discovering_with_an_empty_library_is_still_announced() {
    let (mut g, source) = game(&[]);
    let events = discover(&mut g, source);
    assert_eq!(discovered(&events), vec![(YOU, 2)], "{events:?}");
}
