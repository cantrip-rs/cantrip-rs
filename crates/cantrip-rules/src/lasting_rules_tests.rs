//! Rules a resolving spell or ability gives an open set of objects (CR 611.2c, `EffectKind::LastingRule`): asked again each
//! time they are read, so a creature that arrives after the effect began is under it, one that stops matching is not, and
//! one that loses its abilities still is; the source may be gone. Falter's "creatures without flying can't block this turn",
//! Goblin Diplomats' "each creature attacks this turn if able", Exhaustion's "creatures and lands target opponent controls
//! don't untap during their next untap step" (only in that player's untap step, and ended by it).
//! Beside them, the per-object "it doesn't untap during its controller's next untap step"
//! (`EffectKind::DoesntUntap`, Tangle's), which follows the permanent to a new controller (the Tamiyo, the Moon Sage ruling).
//! Over test cards.

use crate::turn::untap_step;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn without_flying(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && !q.chars(id).has(Keyword::Flying)
}
fn creatures(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}
/// "Creatures and lands [that player] controls."
fn creatures_and_lands_of(q: &dyn Query, _: &Asker, about: Target, id: ObjectId) -> bool {
    (q.chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Land)) && about.player() == Some(q.controller(id))
}

const CANT_BLOCK: u8 = 0;
const MUST_ATTACK: u8 = 1;
const DOESNT_UNTAP: u8 = 2;

static RULES: CardDef = CardDef {
    name: "Lasting Rule Test Rules",
    types: TypeSet::of(&[CardType::Instant]),
    lasting_rules: &[
        LastingRule { text: "Creatures without flying can't block this turn.", rule: LastingRuleKind::CantBlock, applies: without_flying },
        LastingRule { text: "Each creature attacks this turn if able.", rule: LastingRuleKind::MustAttack, applies: creatures },
        LastingRule {
            text: "Creatures and lands target opponent controls don't untap during their next untap step.",
            rule: LastingRuleKind::DoesntUntap,
            applies: creatures_and_lands_of,
        },
    ],
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Lasting Rule Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
const fn creature(name: &'static str, keywords: Keywords) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), keywords, ..CardDef::BLANK }
}
static BEAR: CardDef = creature("Lasting Rule Test Bear", Keywords::NONE);
static FLYER: CardDef = creature("Lasting Rule Test Flyer", Keywords::of(&[Keyword::Flying]));

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&RULES, &FILLER, &BEAR, &FLYER])));
    let deck = vec![table.id_of(&FILLER); 30];
    let mut g = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    g.verify = true;
    g
}

fn permanent(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

/// The `index`th rule of `RULES` from P0's spell, which is gone to the graveyard already: its source has left (CR 611.2a).
fn rule(g: &mut Game, index: u8, about: Target, duration: Duration) {
    let spell = g.state.push_obj(Obj::new(g.cards.id_of(&RULES), P0, Zone::graveyard(P0)));
    g.state.zone_push(Zone::graveyard(P0), spell);
    let timestamp = g.state.timestamp();
    let kind = EffectKind::LastingRule { card: g.cards.id_of(&RULES), index, about };
    g.state.effect_push(ContinuousEffect { source: spell, controller: P0, timestamp, affected: spell, kind, duration, words: Words::NONE });
}

fn effect(g: &mut Game, on: ObjectId, kind: EffectKind, duration: Duration) {
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source: on, controller: P1, timestamp, affected: on, kind, duration, words: Words::NONE });
}

/// Plays turn 1 (P0's) to its declaration of attackers, which `attack` answers, and returns that question and the next
/// declaration of blockers, if P1 is asked one.
fn combat(g: &mut Game, attack: impl Fn(&Request) -> Vec<(ObjectId, Target)>) -> (Request, Option<Request>) {
    let mut attackers = None;
    for _ in 0..200 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        if attackers.is_some() && (matches!(request.kind, RequestKind::DeclareBlockers { .. }) || g.state.turn.step() > Step::DeclareBlockers) {
            let blocks = matches!(request.kind, RequestKind::DeclareBlockers { .. }).then_some(request);
            return (attackers.unwrap(), blocks);
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => {
                attackers = Some(request.clone());
                Answer::Attack(attack(&request))
            }
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    panic!("no combat");
}

/// Falter: a creature without flying that arrives after the rule began can't block either; a flyer can; and a creature
/// that loses its abilities is still under it, the rule being no ability of its.
#[test]
fn a_rule_about_creatures_without_flying_reaches_one_that_arrives_later() {
    let mut g = game();
    let attacker = permanent(&mut g, &BEAR, P0);
    let early = permanent(&mut g, &BEAR, P1);
    rule(&mut g, CANT_BLOCK, Target::None, Duration::EndOfTurn);
    let late = permanent(&mut g, &BEAR, P1);
    let flyer = permanent(&mut g, &FLYER, P1);
    effect(&mut g, early, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    let (_, blocks) = combat(&mut g, |_| vec![(attacker, Target::Player(P1))]);
    let RequestKind::DeclareBlockers { eligible, .. } = blocks.expect("the flyer may block").kind else { unreachable!() };
    assert_eq!(eligible, vec![flyer], "{early:?} and {late:?} can't block");
}

/// The same rule, ended: nothing is barred.
#[test]
fn a_rule_that_has_ended_bars_nothing() {
    let mut g = game();
    let attacker = permanent(&mut g, &BEAR, P0);
    let bear = permanent(&mut g, &BEAR, P1);
    rule(&mut g, CANT_BLOCK, Target::None, Duration::EndOfTurn);
    g.state.effects_retain(|e| !matches!(e.kind, EffectKind::LastingRule { .. }));
    let (_, blocks) = combat(&mut g, |_| vec![(attacker, Target::Player(P1))]);
    let RequestKind::DeclareBlockers { eligible, .. } = blocks.expect("asked").kind else { unreachable!() };
    assert_eq!(eligible, vec![bear]);
}

/// Goblin Diplomats: each creature that arrives later, and one that has lost its abilities, must attack - one requirement
/// for each, an effect and no ability; two such rules are two requirements each.
#[test]
fn each_creature_attacks_if_able_whenever_it_arrived() {
    let mut g = game();
    let early = permanent(&mut g, &BEAR, P0);
    rule(&mut g, MUST_ATTACK, Target::None, Duration::EndOfTurn);
    rule(&mut g, MUST_ATTACK, Target::None, Duration::EndOfTurn);
    let late = permanent(&mut g, &BEAR, P0);
    effect(&mut g, early, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    let (attackers, _) = combat(&mut g, |r| {
        let RequestKind::DeclareAttackers { requirements, .. } = &r.kind else { unreachable!() };
        requirements.suggested.clone()
    });
    let RequestKind::DeclareAttackers { requirements, .. } = &attackers.kind else { unreachable!() };
    let required: Vec<ObjectId> = requirements.required.iter().map(|r| r.attacker).collect();
    assert_eq!(required, vec![early, early, late, late]);
    assert_eq!(requirements.most, 4);
}

fn untap_step_of(g: &mut Game, player: PlayerId) {
    g.state.update_turn(|t| {
        t.active = player;
        t.set_step(Step::Untap);
    });
    let mut cx = Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    untap_step(&mut cx, player).expect("nothing is asked");
}

fn tapped(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = permanent(g, def, controller);
    g.state.update_obj(id, |o| o.tapped = true);
    id
}

fn lasting(g: &Game) -> usize {
    g.state.effects.iter().filter(|e| matches!(e.kind, EffectKind::LastingRule { .. } | EffectKind::DoesntUntap)).count()
}

/// Exhaustion's "creatures and lands target opponent controls don't untap during their next untap step": P0's untap step
/// is not P1's and neither untaps P1's permanents nor ends the rule; P1's next untap step leaves their creature that
/// arrived after it, and their land, tapped, and ends it; the one after untaps them.
#[test]
fn a_rule_about_untapping_holds_in_that_players_next_untap_step_and_ends_with_it() {
    let mut g = game();
    rule(&mut g, DOESNT_UNTAP, Target::Player(P1), Duration::DuringNextUntapStepOf(P1));
    let bear = tapped(&mut g, &BEAR, P1);
    let land = tapped(&mut g, &FILLER, P1);
    let mine = tapped(&mut g, &BEAR, P0);
    untap_step_of(&mut g, P0);
    assert!(!g.state.obj(mine).tapped, "P0's own creature is outside the set");
    assert_eq!(lasting(&g), 1, "P0's untap step is not P1's");
    effect(&mut g, bear, EffectKind::LoseAllAbilities, Duration::Indefinite);
    untap_step_of(&mut g, P1);
    assert!(g.state.obj(bear).tapped && g.state.obj(land).tapped);
    assert_eq!(lasting(&g), 0, "it ends with that untap step");
    untap_step_of(&mut g, P1);
    assert!(!g.state.obj(bear).tapped && !g.state.obj(land).tapped);
}

/// Tangle's "doesn't untap during its controller's next untap step", on P1's creature that P0 then takes: P1's untap step
/// is no longer its controller's and untaps nothing of P0's; P0's next is, leaving it tapped and ending the effect (the
/// Tamiyo, the Moon Sage ruling); P0's one after untaps it.
#[test]
fn doesnt_untap_during_its_controllers_next_untap_step_follows_a_change_of_control() {
    let mut g = game();
    let bear = tapped(&mut g, &BEAR, P1);
    let other = tapped(&mut g, &BEAR, P0);
    effect(&mut g, bear, EffectKind::DoesntUntap, Duration::DuringItsControllersNextUntapStep);
    {
        let mut cx = Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        cantrip_api::Game::add_effect(&mut cx, bear, bear, EffectKind::Control(P0), Duration::Indefinite);
    }
    assert_eq!(g.state.obj(bear).controller, P0);
    untap_step_of(&mut g, P1);
    assert_eq!(lasting(&g), 1, "P1 no longer controls it");
    untap_step_of(&mut g, P0);
    assert!(g.state.obj(bear).tapped, "its new controller's next untap step");
    assert!(!g.state.obj(other).tapped);
    assert_eq!(lasting(&g), 0);
    untap_step_of(&mut g, P0);
    assert!(!g.state.obj(bear).tapped);
}

/// Sleep's "don't untap during that player's next untap step", on P1's creature that P0 then takes: P0's untap step is not
/// that player's, so it untaps; the effect waits for P1's.
#[test]
fn doesnt_untap_during_a_players_next_untap_step_holds_in_that_players_alone() {
    let mut g = game();
    let bear = tapped(&mut g, &BEAR, P0);
    effect(&mut g, bear, EffectKind::DoesntUntap, Duration::DuringNextUntapStepOf(P1));
    untap_step_of(&mut g, P0);
    assert!(!g.state.obj(bear).tapped);
    assert_eq!(lasting(&g), 1);
    untap_step_of(&mut g, P1);
    assert_eq!(lasting(&g), 0);
}
