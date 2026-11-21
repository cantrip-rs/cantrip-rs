//! Bending is announced as `Event::Bent` (CR 701.65b, 701.66b, 701.67c, 702.189b), for "whenever you waterbend, earthbend,
//! firebend, or airbend" and for "if you've done all four this turn": a waterbend ability's cost paid, an earthbend done (even if
//! some of it was impossible), an airbend that exiles one or more objects, and a firebending ability that resolves. Test-only
//! cards: a creature with a waterbend ability, an earthbender, an airbender, and two firebenders. (A waterbend cost a spell or
//! a resolving instruction pays is in `waterbend_cost_tests`.)

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, earthbend, firebending, targets};
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static LAND: CardDef = CardDef { name: "Bent Event Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static BEAR: CardDef = CardDef { name: "Bent Event Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
/// "Waterbend {2}: Nothing."
static WATERBENDER: CardDef = CardDef {
    name: "Bent Event Waterbender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(cost("{2}"), nothing, "Waterbend {2}: Nothing.").waterbend()],
    ..CardDef::BLANK
};
fn exile_the_land(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(land) = item.target(0) {
        g.exile_all(&[land]);
    }
    Ok(())
}
/// "{0}: Earthbend 1." and "{0}: Exile target land you control." (to take the earthbend's target away in response)
static EARTHBENDER: CardDef = CardDef {
    name: "Bent Event Earthbender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[
        ability(Cost::NONE, earthbend::effect::<1>, "{0}: Earthbend 1.").targets(&[earthbend::TARGET]),
        ability(Cost::NONE, exile_the_land, "{0}: Exile target land you control.").targets(&[targets::LAND_YOU_CONTROL]),
    ],
    delayed: &[earthbend::RETURN],
    ..CardDef::BLANK
};
/// "{0}: Airbend up to one target creature."
static AIRBENDER: CardDef = CardDef {
    name: "Bent Event Airbender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(Cost::NONE, airbend_targets, "{0}: Airbend up to one target creature.").targets(&[targets::CREATURE.between(0, 1)])],
    ..CardDef::BLANK
};
/// "Firebending 2."
static FIREBENDER: CardDef = CardDef {
    name: "Bent Event Firebender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[firebending::trigger::<2>("Firebending 2.")],
    ..CardDef::BLANK
};
/// "Firebending X, where X is this creature's power": a power of 0, so nothing is added.
static POWERLESS: CardDef = CardDef {
    name: "Bent Event Powerless Firebender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(1),
    abilities: &[firebending::trigger_its_power("Firebending X, where X is this creature's power.")],
    ..CardDef::BLANK
};

/// A game at your first main phase, nothing in any library but lands, with `cards` on your battlefield (each owned and controlled
/// by you), no summoning sickness.
fn game(cards: &[&'static CardDef]) -> (Game, Vec<ObjectId>) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LAND, &BEAR, &WATERBENDER, &EARTHBENDER, &AIRBENDER, &FIREBENDER, &POWERLESS])));
    let land = table.id_of(&LAND);
    let mut g = Game::new(table, &Setup { decks: vec![vec![land; 20], vec![land; 20]], ..Default::default() });
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
    let ids = cards.iter().map(|def| put(&mut g, def, YOU)).collect();
    (g, ids)
}

fn put(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let mut obj = Obj::new(g.cards.id_of(def), controller, Zone::BATTLEFIELD);
    obj.timestamp = g.state.timestamp();
    let id = g.state.push_obj(obj);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn bends(g: &Game) -> Vec<(PlayerId, Bending)> {
    g.log.iter().filter_map(|r| if let Event::Bent { player, bending } = r.event { Some((player, bending)) } else { None }).collect()
}

/// Activates `source`'s `index`th ability with `targets`, leaving it on the stack.
fn put_on_stack(g: &mut Game, source: ObjectId, index: u8, targets: Vec<Target>) {
    g.run();
    let r = g.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("unexpected {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source) && o.index == index).expect("the ability is offered");
    g.answer(r.id, r.player, Answer::Act(Action { targets, ..Action::of(option) })).unwrap();
}

/// Activates `source`'s only (first) ability with `targets` and plays the stack out.
fn activate(g: &mut Game, source: ObjectId, targets: Vec<Target>) {
    put_on_stack(g, source, 0, targets);
    resolve(g);
}

fn resolve(g: &mut Game) {
    for _ in 0..30 {
        g.run();
        let r = g.pending().cloned().expect("a question");
        match &r.kind {
            RequestKind::Priority { .. } if g.state.stack.is_empty() => return,
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).unwrap(),
            other => panic!("unexpected {other:?}"),
        }
    }
    panic!("the stack never emptied");
}

/// CR 701.66b: an earthbend is recorded as the delayed triggered ability is created: the land is a creature with its counter by then.
#[test]
fn an_earthbend_is_recorded() {
    let (mut g, ids) = game(&[&EARTHBENDER, &LAND]);
    assert!(bends(&g).is_empty());
    activate(&mut g, ids[0], vec![Target::Object(ids[1])]);
    assert_eq!(bends(&g), vec![(YOU, Bending::Earth)]);
    assert_eq!(g.state.obj(ids[1]).counters(CounterKind::PlusOne), 1);
}

/// CR 608.2b: the earthbend's target has left by the time it resolves, so it does not resolve: nothing is earthbent and nothing
/// is recorded.
#[test]
fn an_earthbend_whose_target_is_gone_is_not_recorded() {
    let (mut g, ids) = game(&[&EARTHBENDER, &LAND]);
    put_on_stack(&mut g, ids[0], 0, vec![Target::Object(ids[1])]);
    put_on_stack(&mut g, ids[0], 1, vec![Target::Object(ids[1])]);
    resolve(&mut g);
    assert!(!g.state.battlefield().contains(&ids[1]), "the land was exiled in response");
    assert!(bends(&g).is_empty(), "{:?}", bends(&g));
}

/// CR 701.65b: airbending a creature exiles it: an airbend by you, whoever's creature it was.
#[test]
fn an_airbend_that_exiles_a_creature_is_recorded() {
    let (mut g, ids) = game(&[&AIRBENDER]);
    let theirs = put(&mut g, &BEAR, THEM);
    activate(&mut g, ids[0], vec![Target::Object(theirs)]);
    assert!(!g.state.battlefield().contains(&theirs), "the creature was exiled");
    assert_eq!(bends(&g), vec![(YOU, Bending::Air)]);
}

/// CR 701.65b: "up to one" chosen as none exiles nothing, so there is no airbend.
#[test]
fn an_airbend_of_nothing_is_not_recorded() {
    let (mut g, ids) = game(&[&AIRBENDER]);
    activate(&mut g, ids[0], vec![]);
    assert!(bends(&g).is_empty(), "{:?}", bends(&g));
}

/// CR 702.189b: a firebending ability resolving is a firebend; its controller bends, once for each such ability. (Declared
/// attackers: both creatures, each of whose firebending triggers.)
#[test]
fn each_firebending_ability_that_resolves_is_recorded() {
    let (mut g, _) = game(&[&FIREBENDER, &FIREBENDER, &POWERLESS]);
    for _ in 0..60 {
        g.run();
        let r = g.pending().cloned().expect("a question");
        match &r.kind {
            RequestKind::DeclareAttackers { eligible, .. } => {
                let attacks = eligible.iter().map(|&id| (id, Target::Player(THEM))).collect();
                g.answer(r.id, r.player, Answer::Attack(attacks)).unwrap();
            }
            RequestKind::OrderTriggers { abilities, .. } => g.answer(r.id, r.player, Answer::Order((0..abilities.len() as u32).collect())).unwrap(),
            RequestKind::Priority { .. } if bends(&g).len() == 3 => break,
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).unwrap(),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(bends(&g), vec![(YOU, Bending::Fire); 3], "two Firebending 2 and a firebending that adds nothing (power 0)");
    assert_eq!(g.state.player(YOU).pool.total(), 4);
}

/// CR 701.67c: paying the cost of an activated waterbend ability is a waterbend, by the player who paid, however it was paid.
#[test]
fn a_waterbend_ability_activated_is_recorded() {
    let (mut g, ids) = game(&[&WATERBENDER, &BEAR, &BEAR]);
    g.run();
    let r = g.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("unexpected {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(ids[0])).expect("offered: two Bears can pay {2}").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).unwrap();
    g.run();
    for &bear in &ids[1..] {
        let r = g.pending().cloned().expect("a question");
        let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected a payment, got {:?}", r.kind) };
        let tap = options.iter().find(|o| o.kind == ActionKind::TapToPay && o.index == PAY_WATERBEND && o.source == Some(bear)).expect("a Bear may be tapped").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&tap))).unwrap();
        g.run();
    }
    while let Some(RequestKind::PayCosts { options, .. }) = g.pending().map(|r| &r.kind) {
        let confirm = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the payment is finished").clone();
        let r = g.pending().cloned().unwrap();
        g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).unwrap();
        g.run();
    }
    assert_eq!(bends(&g), vec![(YOU, Bending::Water)]);
    assert!(ids[1..].iter().all(|&b| g.state.obj(b).tapped));
}
