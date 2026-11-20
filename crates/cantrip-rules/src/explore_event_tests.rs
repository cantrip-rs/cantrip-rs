//! An explore (CR 701.44) is announced as `Event::Explored` once it has been carried out (CR 701.44b), naming the permanent
//! that explored, the controller it had (read as it last was where the permanent has left the battlefield, CR 701.44c) and
//! whether the revealed card was a land card, a nonland card, or none because the library was empty (CR 701.44a-b). Test-only
//! cards: a creature whose ability explores, and one that sacrifices itself to explore.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn explores(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::explore(g, item.object)
}

static EXPLORER: CardDef = CardDef {
    name: "Explore Event Explorer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(Cost::NONE, explores, "This creature explores.")],
    ..CardDef::BLANK
};
static MARTYR: CardDef = CardDef {
    name: "Explore Event Martyr",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(SACRIFICE_THIS, explores, "Sacrifice this creature: It explores.")],
    ..CardDef::BLANK
};
static SPELL: CardDef = CardDef { name: "Explore Event Spell", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };
static LAND: CardDef = CardDef { name: "Explore Event Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A game at your first main phase with a library of nonland cards, and whatever `top` is put on top of it.
fn game(top: Option<&'static CardDef>) -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&EXPLORER, &MARTYR, &SPELL, &LAND])));
    let spell = table.id_of(&SPELL);
    let mut g = Game::new(table, &Setup { decks: vec![vec![spell; 20], vec![spell; 20]], ..Default::default() });
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
    if let Some(top) = top {
        let id = g.state.push_obj(Obj::new(g.cards.id_of(top), YOU, Zone::library(YOU)));
        g.state.zone_insert(Zone::library(YOU), 0, id);
    }
    g
}

fn empty_the_library(g: &mut Game) {
    while let Some(&card) = g.state.zone(Zone::library(YOU)).first() {
        g.state.zone_remove(Zone::library(YOU), card);
    }
}

fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let mut obj = Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD);
    obj.timestamp = g.state.timestamp();
    let id = g.state.push_obj(obj);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

/// Activates `source`'s ability and plays the stack out, answering the explore's question of the nonland card with
/// `bin_it`; the events recorded meanwhile.
fn explore_with(g: &mut Game, source: ObjectId, bin_it: bool) -> Vec<Event> {
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
            RequestKind::May { .. } => {
                if bin_it {
                    Answer::Yes
                } else {
                    Answer::No
                }
            }
            other => panic!("unexpected {other:?}"),
        };
        g.answer(r.id, r.player, answer).unwrap();
    }
    g.log[start..].iter().map(|r| r.event).collect()
}

fn explored(events: &[Event]) -> Vec<(ObjectId, PlayerId, ExploreReveal)> {
    events.iter().filter_map(|e| if let Event::Explored { permanent, player, revealed } = *e { Some((permanent, player, revealed)) } else { None }).collect()
}

#[test]
fn exploring_a_land_card_is_announced_as_exploring_a_land_card() {
    let mut g = game(Some(&LAND));
    let creature = put(&mut g, &EXPLORER);
    let events = explore_with(&mut g, creature, false);
    assert_eq!(explored(&events), vec![(creature, YOU, ExploreReveal::Land)], "{events:?}");
    assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), 0);
    // The explore is announced once it is carried out: the land is in the hand already.
    let land = g.state.zone(Zone::hand(YOU)).iter().copied().find(|&c| g.state.obj(c).card == g.cards.id_of(&LAND));
    assert!(land.is_some(), "the land card is in your hand");
}

#[test]
fn exploring_a_nonland_card_is_announced_whether_it_stays_or_goes_to_the_graveyard() {
    for bin_it in [false, true] {
        let mut g = game(None);
        let creature = put(&mut g, &EXPLORER);
        let events = explore_with(&mut g, creature, bin_it);
        assert_eq!(explored(&events), vec![(creature, YOU, ExploreReveal::Nonland)], "bin {bin_it}: {events:?}");
        assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), 1, "the creature gets a +1/+1 counter (CR 701.44a)");
    }
}

#[test]
fn exploring_with_an_empty_library_reveals_nothing_and_still_explores() {
    let mut g = game(None);
    empty_the_library(&mut g);
    let creature = put(&mut g, &EXPLORER);
    let events = explore_with(&mut g, creature, false);
    // CR 701.44a-b: with no card revealed it is "otherwise": the creature gets the counter, and it has explored.
    assert_eq!(explored(&events), vec![(creature, YOU, ExploreReveal::Nothing)], "{events:?}");
    assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), 1);
}

#[test]
fn a_creature_that_left_the_battlefield_still_explored_as_its_controller_had_it() {
    let mut g = game(None);
    let creature = put(&mut g, &MARTYR);
    let events = explore_with(&mut g, creature, false);
    // The creature was sacrificed to explore, so there is nothing to put the counter on (CR 701.44c), and the card is
    // revealed all the same.
    let seen = explored(&events);
    assert_eq!(seen.len(), 1, "{events:?}");
    assert_eq!((seen[0].1, seen[0].2), (YOU, ExploreReveal::Nonland));
    assert!(!g.state.battlefield().contains(&creature), "it was sacrificed");
}
