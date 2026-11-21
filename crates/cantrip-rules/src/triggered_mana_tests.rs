//! A triggered mana ability (CR 605.1b) whose event is not a tap: "whenever you sacrifice an artifact, add {R}" (Crime
//! Novelist). It is a mana ability when it triggers from the activation or resolution of an activated mana ability - a
//! Treasure's sacrifice is paid as its cost - and then resolves at once, as part of that ability (CR 605.4a), no trigger on the
//! stack. The same sacrifice paying the cost of an ability that is not a mana ability triggers it as any triggered ability
//! does, onto the stack (CR 603.3).

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "{1}: Add {G}." (its cost is paid by a mana ability of another permanent)
static PLATE: CardDef = CardDef {
    name: "Sacrifice Test Plate",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}"), add(Mana::G), "{1}: Add {G}.")],
    ..CardDef::BLANK
};

/// "{1}, Sacrifice this artifact: Add {G}." (paid by another, and sacrificing itself)
static SPENT_PLATE: CardDef = CardDef {
    name: "Sacrifice Test Spent Plate",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}").and(SACRIFICE_THIS), add(Mana::G), "{1}, Sacrifice this artifact: Add {G}.")],
    ..CardDef::BLANK
};

/// "{1}, then sacrifice this artifact: Add {G}." (a cost of parts paid in order: the sacrifice after the mana, after a mana
/// ability inside it has paid that)
static PLATE_PAID_FIRST: CardDef = CardDef {
    name: "Sacrifice Test Plate Paid First",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sequence(&[cost("{1}"), SACRIFICE_THIS]), add(Mana::G), "{1}, Sacrifice this artifact: Add {G}.")],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Sacrifice Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Sacrifice this artifact: Add {C}." (a Treasure's kind)
static COIN: CardDef = CardDef {
    name: "Sacrifice Test Coin",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(SACRIFICE_THIS, add(Mana::C), "Sacrifice this artifact: Add {C}.")],
    ..CardDef::BLANK
};

/// "Sacrifice this artifact: Nothing happens." (a Clue's kind: an ability that uses the stack)
static CLUE: CardDef = CardDef {
    name: "Sacrifice Test Clue",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(SACRIFICE_THIS, nothing, "Sacrifice this artifact: Nothing happens.")],
    ..CardDef::BLANK
};

/// "Sacrifice this creature: Add {C}." (not an artifact)
static BEAR: CardDef = CardDef {
    name: "Sacrifice Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[mana_ability(SACRIFICE_THIS, add(Mana::C), "Sacrifice this creature: Add {C}.")],
    ..CardDef::BLANK
};

fn you_sacrifice_an_artifact(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, reason: ZoneReason::Sacrifice, .. }
            if from.is(ZoneKind::Battlefield) && q.controller(*old) == asker.controller && q.chars(*old).is(CardType::Artifact) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn add_red(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}

/// "Whenever you sacrifice an artifact, add {R}."
static NOVELIST: CardDef = CardDef {
    name: "Sacrifice Test Novelist",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(3),
    abilities: &[triggered(when_asked(EventMask::ZONE_CHANGE, you_sacrifice_an_artifact), add_red, "Whenever you sacrifice an artifact, add {R}.")
        .on_sacrifice()
        .mana_ability(TriggeredMana::One(Mana::R))],
    ..CardDef::BLANK
};

/// Your precombat main phase with priority and the Novelist on the battlefield.
fn game() -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &PLATE, &SPENT_PLATE, &PLATE_PAID_FIRST, &COIN, &CLUE, &BEAR, &NOVELIST])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    let novelist = put(&mut g, &NOVELIST, YOU);
    (g, novelist)
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// Activates `source`'s one ability and gives back the question after it.
fn activate(g: &mut Game, source: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(source)).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    next(g)
}

fn pool(g: &Game, mana: Mana) -> u32 {
    g.state.player(YOU).pool.get(mana)
}

fn triggers_on_the_stack(g: &Game, source: ObjectId) -> usize {
    g.state.stack.iter().filter(|i| i.object == source).count()
}

#[test]
fn a_sacrifice_paying_a_mana_ability_triggers_a_triggered_mana_ability_that_resolves_at_once() {
    let (mut g, novelist) = game();
    let coin = put(&mut g, &COIN, YOU);
    let r = activate(&mut g, coin);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "no question: {:?}", r.kind);
    assert_eq!((pool(&g, Mana::C), pool(&g, Mana::R)), (1, 1), "the coin's {{C}} and the Novelist's {{R}} (CR 605.4a)");
    assert_eq!(triggers_on_the_stack(&g, novelist), 0, "a mana ability doesn't use the stack (CR 605.3b, 605.4a)");
    assert!(g.state.stack.is_empty());
    assert!(g.state.obj(coin).became.is_some(), "the coin was sacrificed");
}

#[test]
fn the_same_sacrifice_for_an_ability_that_uses_the_stack_triggers_it_onto_the_stack() {
    let (mut g, novelist) = game();
    let clue = put(&mut g, &CLUE, YOU);
    let r = activate(&mut g, clue);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "{:?}", r.kind);
    // The clue's own ability and the Novelist's trigger, which is no mana ability here (CR 605.1b, 603.3): not resolved yet.
    assert_eq!(triggers_on_the_stack(&g, novelist), 1, "an ordinary triggered ability waits on the stack");
    assert_eq!(pool(&g, Mana::R), 0);
}

#[test]
fn a_sacrifice_that_does_not_match_triggers_nothing() {
    // Not an artifact.
    let (mut g, novelist) = game();
    let bear = put(&mut g, &BEAR, YOU);
    activate(&mut g, bear);
    assert_eq!((pool(&g, Mana::C), pool(&g, Mana::R), triggers_on_the_stack(&g, novelist)), (1, 0, 0));

    // Another player's artifact: the ability asks "you sacrifice".
    let (mut g, novelist) = game();
    let coin = put(&mut g, &COIN, THEM);
    g.state.update_obj(coin, |o| o.controller = YOU);
    g.state.update_obj(novelist, |o| o.controller = THEM);
    activate(&mut g, coin);
    assert_eq!((pool(&g, Mana::R), triggers_on_the_stack(&g, novelist)), (0, 0));
}

#[test]
fn a_mana_ability_paid_by_another_triggers_only_once() {
    // The Plate's {1} is paid by the Coin, a mana ability inside the Plate's own activation: the Coin's sacrifice triggers the
    // Novelist when the Coin's ability ends, and the Plate, looking back over its history, doesn't count it again (CR 605.4a).
    let (mut g, novelist) = game();
    let plate = put(&mut g, &PLATE, YOU);
    let coin = put(&mut g, &COIN, YOU);
    let mut r = activate(&mut g, plate);
    while !matches!(r.kind, RequestKind::Priority { .. }) {
        let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("unexpected question {:?}", r.kind) };
        // Pay it by the coin, a mana ability of its own, pay with its {C}, then confirm.
        let step = options.iter().find(|o| o.source == Some(coin)).or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana && o.mana == Some(Mana::C))).or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)).unwrap_or_else(|| panic!("nothing to do in {options:?}")).clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&step))).expect("a legal answer");
        r = next(&mut g);
    }
    assert!(g.state.obj(coin).became.is_some(), "the coin paid for it");
    assert_eq!((pool(&g, Mana::G), pool(&g, Mana::R)), (1, 1), "the Plate's {{G}} and one {{R}} for the one sacrifice");
    assert_eq!(triggers_on_the_stack(&g, novelist), 0);
}

#[test]
fn a_mana_ability_paid_by_another_still_triggers_for_its_own_sacrifice() {
    // The Spent Plate's own sacrifice is its part of the cost, beside the Coin's, a mana ability inside its activation: each
    // sacrifice triggers the Novelist once, whichever comes first (CR 605.4a).
    for def in [&SPENT_PLATE, &PLATE_PAID_FIRST] {
        its_own_sacrifice_triggers_beside_the_coins(def);
    }
}

/// `def`'s {1} paid by the Coin, its own sacrifice before (Spent Plate) or right after (Plate Paid First) the Coin's span.
fn its_own_sacrifice_triggers_beside_the_coins(def: &'static CardDef) {
    let (mut g, novelist) = game();
    let plate = put(&mut g, def, YOU);
    let coin = put(&mut g, &COIN, YOU);
    let mut r = activate(&mut g, plate);
    while !matches!(r.kind, RequestKind::Priority { .. }) {
        let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("unexpected question {:?}", r.kind) };
        let step = options
            .iter()
            .find(|o| o.source == Some(coin))
            .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana && o.mana == Some(Mana::C)))
            .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
            .unwrap_or_else(|| panic!("nothing to do in {options:?}"))
            .clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&step))).expect("a legal answer");
        r = next(&mut g);
    }
    assert!(g.state.obj(coin).became.is_some() && g.state.obj(plate).became.is_some(), "both were sacrificed");
    assert_eq!((pool(&g, Mana::G), pool(&g, Mana::R)), (1, 2), "the Plate's {{G}} and one {{R}} for each of the two sacrifices");
    assert_eq!(triggers_on_the_stack(&g, novelist), 0);
}
