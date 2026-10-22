//! CR 800.4e/506.4c/510.1b: absent defenders never appear in divisions or combat assignments.

use super::*;
use crate::Game as RulesGame;

const YOU: PlayerId = PlayerId(0);
const LEFT: PlayerId = PlayerId(1);
const OTHER: PlayerId = PlayerId(2);

static ATTACKER: CardDef = CardDef {
    name: "Departed combat trampler probe",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Lifelink]),
    ..CardDef::BLANK
};
static FIRST: CardDef = CardDef {
    name: "Departed combat first strike probe",
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Lifelink, Keyword::FirstStrike]),
    ..ATTACKER
};
static DOUBLE: CardDef = CardDef {
    name: "Departed combat double strike probe",
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Lifelink, Keyword::DoubleStrike]),
    ..ATTACKER
};
static BLOCKER: CardDef = CardDef { name: "Departed combat blocker probe", power: Some(1), toughness: Some(1), keywords: Keywords::NONE, ..ATTACKER };
static WALKER: CardDef = CardDef { name: "Departed combat walker probe", types: TypeSet::of(&[CardType::Planeswalker]), loyalty: Some(10), ..CardDef::BLANK };
static BATTLE: CardDef = CardDef { name: "Departed combat battle probe", types: TypeSet::of(&[CardType::Battle]), defense: Some(10), ..CardDef::BLANK };

fn game(def: &'static CardDef, blocked: bool) -> (RulesGame, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&ATTACKER, &FIRST, &DOUBLE, &BLOCKER, &WALKER, &BATTLE])));
    let mut game = RulesGame::new(cards, &crate::Setup { decks: vec![Vec::new(); 3], ..Default::default() });
    game.state.update_turn(|t| { t.number = 1; t.active = YOU; t.set_step(Step::CombatDamage); t.attacked = true; });
    let attacker = game.state.push_obj(Obj::new(cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, attacker);
    game.state.set_attackers(vec![Attacker { blocked, ..Attacker::new(attacker, Target::Player(LEFT)) }]);
    (game, attacker)
}

fn cx(game: &mut RulesGame) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None }
}

fn damage_events(cx: &Cx) -> usize {
    cx.g.log.iter().filter(|r| matches!(r.event, Event::Damage { combat: true, .. })).count()
}

#[test]
fn departed_player_gets_no_unblocked_or_trample_assignment_in_either_damage_step() {
    for def in [&ATTACKER, &FIRST, &DOUBLE] {
        for blocked in [false, true] {
            let (mut game, _) = game(def, blocked);
            game.state.update_player(LEFT, |p| p.lost = true);
            let mut cx = cx(&mut game);
            assert!(combat_assignments(&cx, &cx.s().attackers, &[], &[]).is_empty(), "not merely damage ignored afterward");
            combat_damage(&mut cx).unwrap();
            if cx.s().turn.damage_round == 1 { combat_damage(&mut cx).unwrap(); }
            assert_eq!(damage_events(&cx), 0);
            assert_eq!(cx.s().player(YOU).life, 20);
            assert_eq!(cx.s().player(LEFT).life, 20);
        }
    }
}

#[test]
fn a_tramplers_living_blockers_still_receive_its_entire_damage_without_a_defender_option() {
    let (mut game, attacker) = game(&ATTACKER, true);
    game.state.update_player(LEFT, |p| p.lost = true);
    let mut blockers = Vec::new();
    for _ in 0..2 {
        let id = game.state.push_obj(Obj::new(game.cards.id_of(&BLOCKER), OTHER, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        blockers.push(id);
    }
    game.state.set_blocks(blockers.iter().map(|&blocker| Block { blocker, attacker, first_strike: false }).collect());
    let cx = cx(&mut game);
    let division = attacker_division(&cx, &cx.s().attackers[0]).unwrap();
    assert!(!division.may_assign_to_defender);
    assert_eq!(division.defender, Target::None);
    assert_eq!(division.suggested_to_defender, 0);
    assert_eq!(division.suggested.iter().sum::<u16>(), 3);
    assert_eq!(division.creatures, blockers);
    let request = Request { id: 0, player: YOU, may_cancel: false, kind: RequestKind::AssignCombatDamage { divisions: vec![division] } };
    assert!(crate::check_answer(cx.g, &request, &Answer::Shares(vec![(attacker, Some(blockers[0]), 1), (attacker, Some(blockers[1]), 1), (attacker, None, 1)])).is_err());
}

#[test]
fn a_single_living_blocker_still_takes_all_damage_when_the_defender_is_gone() {
    let (mut game, attacker) = game(&ATTACKER, true);
    game.state.update_player(LEFT, |p| p.lost = true);
    let blocker = game.state.push_obj(Obj::new(game.cards.id_of(&BLOCKER), OTHER, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, blocker);
    game.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }]);
    let mut cx = cx(&mut game);
    assert!(attacker_division(&cx, &cx.s().attackers[0]).is_none());
    let assignments = combat_assignments(&cx, &cx.s().attackers, &[blocker], &[]);
    assert_eq!(assignments, vec![(attacker, Target::Object(blocker), 3), (blocker, Target::Object(attacker), 1)]);
    combat_damage(&mut cx).unwrap();
    assert_eq!(cx.s().obj(blocker).damage, 3);
    assert_eq!(cx.s().obj(attacker).damage, 1);
    assert_eq!(cx.s().player(YOU).life, 23);
    assert_eq!(cx.s().player(LEFT).life, 20);
}

#[test]
fn live_defenders_still_receive_normal_first_and_double_strike_damage() {
    for (def, dealt) in [(&ATTACKER, 3), (&FIRST, 3), (&DOUBLE, 6)] {
        let (mut game, _) = game(def, false);
        let mut cx = cx(&mut game);
        combat_damage(&mut cx).unwrap();
        if cx.s().turn.damage_round == 1 { combat_damage(&mut cx).unwrap(); }
        assert_eq!(cx.s().player(LEFT).life, 20 - dealt);
        assert_eq!(cx.s().player(YOU).life, 20 + dealt);
    }
}

#[test]
fn absent_planeswalkers_have_no_assignment_but_live_battles_keep_departed_protectors() {
    for def in [&WALKER, &BATTLE] {
        let (mut game, attacker) = game(&ATTACKER, false);
        let id = game.state.push_obj(Obj::new(game.cards.id_of(def), OTHER, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        game.state.update_obj(id, |o| { o.marks.protector = (def.types.has(CardType::Battle)).then_some(LEFT); });
        game.state.set_attackers(vec![Attacker::new(attacker, Target::Object(id))]);
        game.state.update_player(LEFT, |p| p.lost = true);
        let mut cx = cx(&mut game);
        if def.types.has(CardType::Planeswalker) {
            ops::exile_all(&mut cx, &[id]);
            assert!(!combat_defender_present(&cx, Target::Object(id)));
            combat_damage(&mut cx).unwrap();
            assert_eq!(damage_events(&cx), 0);
            assert_eq!(cx.s().player(YOU).life, 20);
        } else {
            assert!(combat_defender_present(&cx, Target::Object(id)));
            combat_damage(&mut cx).unwrap();
            assert_eq!(damage_events(&cx), 1);
            assert_eq!(cx.s().player(YOU).life, 23);
        }
    }
}
