//! Surge and freerunning (CR 702.117a, 702.173a), the alternative costs `cantrip_mechanics::surge` and `freerunning` make,
//! over test cards: each offered only while its condition holds, asked of the event history, and for exactly its cost.
//! Surge: its caster has cast another spell this turn - not an opponent's, not last turn's. Freerunning: a player was dealt
//! combat damage this turn by a creature that, at the time it dealt it, was an Assassin creature under the caster's control
//! (the ruling: whatever has happened to it since).

use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;
use cantrip_mechanics::{freerunning, surge};
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Surge Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Surge {1}{R}": a 3/2 for {3}{R}.
static RUNNER: CardDef = CardDef {
    name: "Surge Test Runner",
    cost: mana("{3}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(2),
    ways: &[surge::way(mana("{1}{R}"), "Surge {1}{R}")],
    ..CardDef::BLANK
};

/// "Freerunning {3}{B}": a 6/3 for {4}{B}.
static AMBUSHER: CardDef = CardDef {
    name: "Surge Test Ambusher",
    cost: mana("{4}{B}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(6),
    toughness: Some(3),
    ways: &[freerunning::way(mana("{3}{B}"), "Freerunning {3}{B}")],
    ..CardDef::BLANK
};

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// An instant for {0} that does nothing.
static TRIFLE: CardDef = CardDef { name: "Surge Test Trifle", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], nothing), ..CardDef::BLANK };

/// A 2/2 Human Assassin.
static ASSASSIN: CardDef = CardDef {
    name: "Surge Test Assassin",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Assassin]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A 2/2 Human Rogue.
static ROGUE: CardDef = CardDef {
    name: "Surge Test Rogue",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Rogue]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A noncreature Assassin: an Assassin Kindred artifact can't be one without being a creature, so an Assassin-typed artifact
/// stands in for an Assassin that isn't a creature (CR 702.173a: "an Assassin creature").
static ASSASSIN_ARTIFACT: CardDef = CardDef {
    name: "Surge Test Assassin Artifact",
    types: TypeSet::of(&[CardType::Kindred, CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Assassin]),
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&FILLER, &RUNNER, &AMBUSHER, &TRIFLE, &ASSASSIN, &ROGUE, &ASSASSIN_ARTIFACT])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}

fn reoffer(g: &mut Game) {
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn add(g: &mut Game, mana: Mana, n: u32) {
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, mana, n, source, None, ManaLasts::Step);
    reoffer(g);
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn options(g: &mut Game) -> Vec<ActionOption> {
    match next(g).kind {
        RequestKind::Priority { options } => options,
        other => panic!("expected priority, got {other:?}"),
    }
}

/// The option to cast `card` by its first way (its surge or freerunning cost), if it is offered.
fn way_cast(g: &mut Game, card: ObjectId) -> Option<ActionOption> {
    options(g).into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == 1)
}

fn take(g: &mut Game, option: &ActionOption) {
    let r = next(g);
    g.answer(r.id, r.player, Answer::Act(Action::of(option))).expect("a legal answer");
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { return };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
}

fn resolve_all(g: &mut Game) {
    while !g.state.stack.is_empty() {
        let r = next(g);
        g.answer(r.id, r.player, Answer::Pass).expect("a legal answer");
    }
    g.run();
}

/// Casts a Trifle of `YOU`'s, and lets it resolve.
fn cast_trifle(g: &mut Game) {
    let trifle = put(g, &TRIFLE, YOU, Zone::hand(YOU));
    let option = options(g).into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(trifle) && o.index == 0).expect("the Trifle");
    take(g, &option);
    resolve_all(g);
}

/// `source` deals `amount` damage to `THEM`, as combat damage or not.
fn hit(g: &mut Game, source: ObjectId, combat: bool) {
    ops::damage(&mut cx(g), source, Target::Player(THEM), 2, combat);
    reoffer(g);
}

/// Makes `id` have `def`'s characteristics from now on (a copy effect's way of changing what it is).
fn become_(g: &mut Game, id: ObjectId, def: &'static CardDef) {
    let values = Copiable::of(g.cards.id_of(def));
    g.state.update_obj(id, |o| o.copy = Some(values));
    reoffer(g);
}

/// CR 702.117a: not offered with no other spell cast this turn, whatever mana there is.
#[test]
fn surge_is_refused_with_no_other_spell_cast_this_turn() {
    let mut g = game();
    let card = put(&mut g, &RUNNER, YOU, Zone::hand(YOU));
    add(&mut g, Mana::R, 4);
    assert!(way_cast(&mut g, card).is_none());
}

/// CR 702.117a: once you have cast another spell this turn, surge costs exactly {1}{R} - {R} alone does not pay it.
#[test]
fn surge_is_offered_for_exactly_its_cost_after_another_spell() {
    let mut g = game();
    let card = put(&mut g, &RUNNER, YOU, Zone::hand(YOU));
    cast_trifle(&mut g);
    add(&mut g, Mana::R, 1);
    assert!(way_cast(&mut g, card).is_none(), "{{R}} does not pay {{1}}{{R}}");
    add(&mut g, Mana::C, 1);
    let option = way_cast(&mut g, card).expect("surge is offered");
    take(&mut g, &option);
    assert_eq!(g.state.player(YOU).pool.total(), 0, "it cost exactly {{1}}{{R}}");
    let spell = g.state.stack.last().expect("the spell").object;
    assert!(surge::surged(&g, spell));
    assert_eq!(Query::mana_value(&g, spell), 4, "an alternative cost doesn't change the mana cost (CR 118.9c)");
}

/// CR 702.117a: an opponent's spell is not "you or one of your teammates" (there are no teams), and last turn's spell is
/// not this turn's.
#[test]
fn surge_ignores_an_opponents_spell_and_last_turns() {
    let mut g = game();
    let card = put(&mut g, &RUNNER, YOU, Zone::hand(YOU));
    let theirs = put(&mut g, &TRIFLE, THEM, Zone::STACK);
    cx(&mut g).record(Event::Cast(cantrip_api::history::Cast {
        spell: theirs,
        player: THEM,
        types: TypeSet::of(&[CardType::Instant]),
        from: ZoneKind::Hand,
        way: 0,
        by: Grant::NONE,
        fact: 0,
    }));
    add(&mut g, Mana::R, 4);
    assert!(way_cast(&mut g, card).is_none(), "an opponent's spell");
    let mut g = game();
    let card = put(&mut g, &RUNNER, YOU, Zone::hand(YOU));
    cast_trifle(&mut g);
    g.state.update_turn(|t| t.number += 1);
    add(&mut g, Mana::R, 4);
    assert!(way_cast(&mut g, card).is_none(), "last turn's spell");
}

/// CR 702.173a: refused with no damage, after an Assassin's noncombat damage, after a non-Assassin's combat damage, after an
/// opponent's Assassin's, and after a noncreature Assassin's.
#[test]
fn freerunning_is_refused_without_an_assassin_creatures_combat_damage() {
    let cases: [(&'static CardDef, PlayerId, bool, &str); 4] = [
        (&ASSASSIN, YOU, false, "noncombat damage"),
        (&ROGUE, YOU, true, "a non-Assassin's combat damage"),
        (&ASSASSIN, THEM, true, "an opponent's Assassin"),
        (&ASSASSIN_ARTIFACT, YOU, true, "an Assassin that is no creature"),
    ];
    let mut g = game();
    let card = put(&mut g, &AMBUSHER, YOU, Zone::hand(YOU));
    add(&mut g, Mana::B, 5);
    assert!(way_cast(&mut g, card).is_none(), "no damage at all");
    for (def, controller, combat, why) in cases {
        let mut g = game();
        let card = put(&mut g, &AMBUSHER, YOU, Zone::hand(YOU));
        let source = put(&mut g, def, controller, Zone::BATTLEFIELD);
        hit(&mut g, source, combat);
        add(&mut g, Mana::B, 5);
        assert!(way_cast(&mut g, card).is_none(), "{why}");
    }
}

/// CR 702.173a: "a player was dealt combat damage" - an Assassin creature's combat damage to a creature is not.
#[test]
fn freerunning_is_refused_after_an_assassins_combat_damage_to_a_creature() {
    let mut g = game();
    let card = put(&mut g, &AMBUSHER, YOU, Zone::hand(YOU));
    let source = put(&mut g, &ASSASSIN, YOU, Zone::BATTLEFIELD);
    let blocker = put(&mut g, &ROGUE, THEM, Zone::BATTLEFIELD);
    ops::damage(&mut cx(&mut g), source, Target::Object(blocker), 1, true);
    reoffer(&mut g);
    add(&mut g, Mana::B, 5);
    assert!(way_cast(&mut g, card).is_none(), "combat damage to a creature");
    hit(&mut g, source, true);
    assert!(way_cast(&mut g, card).is_some(), "and then to a player");
}

/// The ruling: what the creature was as it dealt the damage counts - one that had stopped being an Assassin before it dealt
/// it does not, and a Rogue that becomes an Assassin afterwards does not.
#[test]
fn freerunning_reads_the_creature_as_it_dealt_the_damage() {
    let mut g = game();
    let card = put(&mut g, &AMBUSHER, YOU, Zone::hand(YOU));
    let creature = put(&mut g, &ASSASSIN, YOU, Zone::BATTLEFIELD);
    become_(&mut g, creature, &ROGUE);
    hit(&mut g, creature, true);
    add(&mut g, Mana::B, 5);
    assert!(way_cast(&mut g, card).is_none(), "no longer an Assassin as it dealt the damage");
    let mut g = game();
    let card = put(&mut g, &AMBUSHER, YOU, Zone::hand(YOU));
    let creature = put(&mut g, &ROGUE, YOU, Zone::BATTLEFIELD);
    hit(&mut g, creature, true);
    become_(&mut g, creature, &ASSASSIN);
    add(&mut g, Mana::B, 5);
    assert!(way_cast(&mut g, card).is_none(), "an Assassin only after it dealt the damage");
}

/// CR 702.173a and the ruling: after your Assassin creature's combat damage to a player, freerunning costs exactly {3}{B} -
/// not {2}{B} - whatever happens to the Assassin afterwards: it stops being one, it dies, an opponent gains control of it.
#[test]
fn freerunning_is_offered_for_exactly_its_cost_after_an_assassins_combat_damage() {
    for after in ["nothing", "no longer an Assassin", "died", "changed control"] {
        let mut g = game();
        let card = put(&mut g, &AMBUSHER, YOU, Zone::hand(YOU));
        let creature = put(&mut g, &ASSASSIN, YOU, Zone::BATTLEFIELD);
        hit(&mut g, creature, true);
        match after {
            "no longer an Assassin" => become_(&mut g, creature, &ROGUE),
            "died" => {
                cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[creature]);
                reoffer(&mut g);
            }
            "changed control" => {
                g.state.update_obj(creature, |o| o.controller = THEM);
                reoffer(&mut g);
            }
            _ => {}
        }
        add(&mut g, Mana::B, 1);
        add(&mut g, Mana::C, 2);
        assert!(way_cast(&mut g, card).is_none(), "{{2}}{{B}} does not pay {{3}}{{B}} ({after})");
        add(&mut g, Mana::C, 1);
        let option = way_cast(&mut g, card).unwrap_or_else(|| panic!("freerunning is offered ({after})"));
        take(&mut g, &option);
        assert_eq!(g.state.player(YOU).pool.total(), 0, "it cost exactly {{3}}{{B}} ({after})");
    }
}
