//! Goad (CR 701.15) and "attacks that player this combat if able" (CR 508.1d) are effects, not abilities: a creature under
//! one that then loses all its abilities (CR 613.1f, a later timestamp) still attacks each combat if able. A gained "this
//! creature attacks each combat if able" (`EffectKind::MustAttack`, Carnage's) is an ability, and is lost. Over test cards.

use crate::{Game, Setup};
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Attack Effect Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static BEAR: CardDef =
    CardDef { name: "Attack Effect Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FILLER, &BEAR])))
}

/// P0's Bear under `kind` from P1, then losing all its abilities: the most requirements P0's declaration must obey.
fn required_after_losing_abilities(kind: EffectKind) -> u32 {
    let table = table();
    let deck = vec![table.id_of(&FILLER); 30];
    let mut g = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    g.verify = true;
    let bear = g.state.push_obj(Obj::new(table.id_of(&BEAR), P0, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, bear);
    for (controller, kind) in [(P1, kind), (P1, EffectKind::LoseAllAbilities)] {
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect { source: bear, controller, timestamp, affected: bear, kind, duration: Duration::EndOfTurn, words: Words::NONE });
    }
    for _ in 0..200 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        if let RequestKind::DeclareAttackers { requirements, .. } = &request.kind {
            return requirements.most;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    panic!("no declaration of attackers");
}

#[test]
fn a_goaded_creature_that_loses_its_abilities_is_still_goaded() {
    // A game of two: attacking the goader, the only opponent, obeys "attack"; "a player other than the goader" can't be obeyed.
    assert_eq!(required_after_losing_abilities(EffectKind::Goaded), 1);
}

#[test]
fn a_creature_that_must_attack_a_player_and_loses_its_abilities_still_must() {
    assert_eq!(required_after_losing_abilities(EffectKind::MustAttackAt(Target::Player(P1))), 1);
}

#[test]
fn a_gained_attacks_each_combat_ability_is_lost_with_the_others() {
    assert_eq!(required_after_losing_abilities(EffectKind::MustAttack), 0);
}

const P2: PlayerId = PlayerId(2);

/// A game of three P0 begins, with P1's Bear under "attacks P2 during its controller's next turn if able" from turn 1 (P0's),
/// and P0 taking the Bear for good during turn 1 with `stolen`: each declaration of attackers through turn 7 - its turn,
/// whose it was and the most requirements it had to obey - and how many effects are left after turn 6's cleanup.
fn next_turn_requirements(stolen: bool) -> (Vec<(u32, PlayerId, u32)>, usize) {
    let table = table();
    let deck = vec![table.id_of(&FILLER); 30];
    let mut g = Game::new(table, &Setup { decks: vec![deck.clone(), deck.clone(), deck], ..Default::default() });
    g.verify = true;
    let bear = g.state.push_obj(Obj::new(table.id_of(&BEAR), if stolen { P0 } else { P1 }, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, bear);
    let timestamp = g.state.timestamp();
    let kind = EffectKind::MustAttackAt(Target::Player(P2));
    g.state.effect_push(ContinuousEffect { source: bear, controller: P0, timestamp, affected: bear, kind, duration: Duration::DuringControllersNextTurn(1), words: Words::NONE });
    let mut seen = Vec::new();
    let mut left = usize::MAX;
    for _ in 0..1000 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        let turn = g.state.turn.number;
        if turn == 7 {
            left = g.state.effects.len();
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { requirements, .. } => {
                seen.push((turn, request.player, requirements.most));
                Answer::Attack(requirements.suggested.clone())
            }
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    (seen, left)
}

/// CR 611.2a: P1's Bear must attack P2 during P1's next turn (turn 2) and in no other - not in P1's turn 5 - and the effect
/// is gone once every player has begun a turn since it began (P0's turn 4).
#[test]
fn a_requirement_during_its_controllers_next_turn_applies_in_that_turn_alone() {
    let (seen, left) = next_turn_requirements(false);
    assert_eq!(seen, vec![(2, P1, 1), (5, P1, 0)]);
    assert_eq!(left, 0);
}

/// Gideon, Battle-Forged's ruling: a creature that changes controllers before its controller's next turn is under the
/// requirement during its new controller's next turn. P0 controls the Bear in turn 1, the turn the effect began in, which
/// is never that turn; P0's turn 4 is.
#[test]
fn a_requirement_during_its_controllers_next_turn_follows_a_change_of_control() {
    let (seen, _) = next_turn_requirements(true);
    assert_eq!(seen, vec![(1, P0, 0), (4, P0, 1)]);
}

static HASTY: CardDef = CardDef {
    name: "Attack Effect Test Hasty",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Haste, Keyword::Vigilance]),
    ..CardDef::BLANK
};

/// P2's hasty Bear is P1's until P2's next turn, under "attacks P0 during its controller's next turn if able" from turn 1.
/// P1's turn 2 is its controller's next turn, and it must attack P0 then. P2 gets it back as turn 3 begins: the ruling
/// moves the requirement to a new controller only on a change of control before the creature has had its chance to
/// attack, so P2's turn 3 is not under it (CR 611.2a: the duration is over with that turn).
#[test]
fn a_requirement_during_its_controllers_next_turn_is_spent_with_that_turn_whoever_controls_it_next() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &HASTY])));
    let deck = vec![table.id_of(&FILLER); 30];
    let mut g = Game::new(table, &Setup { decks: vec![deck.clone(), deck.clone(), deck], ..Default::default() });
    g.verify = true;
    let bear = g.state.push_obj(Obj::new(table.id_of(&HASTY), P2, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, bear);
    {
        let mut cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        cantrip_api::Game::add_effect(&mut cx, bear, bear, EffectKind::Control(P1), Duration::UntilNextTurnOf(P2));
    }
    let timestamp = g.state.timestamp();
    let kind = EffectKind::MustAttackAt(Target::Player(P0));
    g.state.effect_push(ContinuousEffect { source: bear, controller: P0, timestamp, affected: bear, kind, duration: Duration::DuringControllersNextTurn(1), words: Words::NONE });
    let mut seen = Vec::new();
    for _ in 0..1000 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        let turn = g.state.turn.number;
        if turn == 4 {
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { requirements, .. } => {
                seen.push((turn, request.player, requirements.most));
                Answer::Attack(requirements.suggested.clone())
            }
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(seen, vec![(2, P1, 1), (3, P2, 0)]);
}

/// P1's Bear is P2's until P2's next turn, under "attacks P0 during its controller's next turn if able" from turn 1 (P0's).
/// P2's next turn would be turn 3, but P1 gets the Bear back as turn 3 begins, before its chance to attack: the ability
/// then applies during its new controller's next turn (Gideon, Battle-Forged's ruling) - P1's turn 5, though P1's turn 2
/// came before P1 had the Bear.
#[test]
fn a_requirement_during_its_controllers_next_turn_moves_to_a_new_controller_whose_turn_already_passed() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &HASTY])));
    let deck = vec![table.id_of(&FILLER); 30];
    let mut g = Game::new(table, &Setup { decks: vec![deck.clone(), deck.clone(), deck], ..Default::default() });
    g.verify = true;
    let bear = g.state.push_obj(Obj::new(table.id_of(&BEAR), P1, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, bear);
    {
        let mut cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        cantrip_api::Game::add_effect(&mut cx, bear, bear, EffectKind::Control(P2), Duration::UntilNextTurnOf(P2));
    }
    let timestamp = g.state.timestamp();
    let kind = EffectKind::MustAttackAt(Target::Player(P0));
    g.state.effect_push(ContinuousEffect { source: bear, controller: P0, timestamp, affected: bear, kind, duration: Duration::DuringControllersNextTurn(1), words: Words::NONE });
    let mut seen = Vec::new();
    for _ in 0..2000 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        let turn = g.state.turn.number;
        if turn == 6 {
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { requirements, eligible, .. } => {
                seen.push((turn, request.player, eligible.contains(&bear), requirements.most));
                Answer::Attack(requirements.suggested.clone())
            }
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    assert!(seen.contains(&(5, P1, true, 1)), "{seen:?}");
    // P1's turn 2, before P1 had it back, asked nothing of it.
    assert!(!seen.iter().any(|s| s.0 == 2 && s.3 > 0), "{seen:?}");
}
