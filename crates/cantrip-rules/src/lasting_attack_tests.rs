//! CR 508.1c and 611.2a-c: fixed-object and open-set attack restrictions, over generic test cards.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
static BEAR: CardDef =
    CardDef { name: "Lasting Attack Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static WALKER: CardDef = CardDef { name: "Lasting Attack Walker", types: TypeSet::of(&[CardType::Planeswalker]), loyalty: Some(5), ..CardDef::BLANK };
static BATTLE: CardDef = CardDef { name: "Lasting Attack Battle", types: TypeSet::of(&[CardType::Battle]), defense: Some(5), ..CardDef::BLANK };
fn every_creature(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}
// The canonical Chronomantic Escape rule, without importing its card or unrelated prerequisites.
static OPEN: CardDef = CardDef {
    name: "Lasting Attack Open Set",
    types: TypeSet::of(&[CardType::Sorcery]),
    lasting_rules: &[LastingRule {
        text: "Until your next turn, creatures can't attack you.",
        rule: LastingRuleKind::CantAttackYou,
        applies: every_creature,
    }],
    ..CardDef::BLANK
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&BEAR, &WALKER, &BATTLE, &OPEN])));
    let mut g = Game::new(cards, &Setup { decks: vec![Vec::new(); 3], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}
fn put(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: Some(P0), producing: None, reached: None }
}
fn counter(g: &mut Game, id: ObjectId, n: i32) {
    crate::counters::put(&mut cx(g), id, CounterKind::Vow, n, P1, crate::counters::How::Effect);
}
fn restrict(g: &mut Game, source: ObjectId, bear: ObjectId, you: PlayerId, walkers: bool) {
    cantrip_api::Game::add_effect(
        &mut cx(g),
        source,
        bear,
        EffectKind::CantAttackPlayer { player: you, planeswalkers: walkers },
        Duration::WhileCounter(CounterKind::Vow),
    );
}
fn barred(g: &Game, bears: &[ObjectId], targets: &[Target]) -> Vec<(ObjectId, Target)> {
    crate::static_rules::lasting_rules_barring_attacks(g, g.cards, bears, targets)
}
#[test]
fn fixed_identity_survives_source_loss_ability_loss_and_stealing_but_planeswalker_control_is_current() {
    let mut g = game();
    let source = put(&mut g, &OPEN, P0);
    let bear = put(&mut g, &BEAR, P1);
    let other = put(&mut g, &BEAR, P1);
    let walker = put(&mut g, &WALKER, P0);
    let battle = put(&mut g, &BATTLE, P0);
    for id in [bear, other] {
        counter(&mut g, id, 1);
    }
    restrict(&mut g, source, bear, P0, true);
    let targets = [Target::Player(P0), Target::Player(P2), Target::Object(walker), Target::Object(battle)];
    assert_eq!(barred(&g, &[bear, other], &targets), vec![(bear, targets[0]), (bear, targets[2])]);
    cantrip_api::Game::add_effect(&mut cx(&mut g), source, bear, EffectKind::LoseAllAbilities, Duration::Indefinite);
    cantrip_api::Game::add_effect(&mut cx(&mut g), source, bear, EffectKind::Control(P2), Duration::Indefinite);
    cantrip_api::Game::exile_all(&mut cx(&mut g), &[source]);
    assert_eq!(g.state.obj(bear).controller, P2);
    assert_eq!(barred(&g, &[bear, other], &targets), vec![(bear, targets[0]), (bear, targets[2])]);
    cantrip_api::Game::add_effect(&mut cx(&mut g), bear, walker, EffectKind::Control(P1), Duration::Indefinite);
    assert_eq!(barred(&g, &[bear], &targets), vec![(bear, targets[0])]);
}
#[test]
fn removing_and_readding_the_last_vow_in_one_instruction_permanently_ends_it() {
    let mut g = game();
    let source = put(&mut g, &OPEN, P0);
    let bear = put(&mut g, &BEAR, P1);
    counter(&mut g, bear, 2);
    restrict(&mut g, source, bear, P0, true);
    crate::counters::remove(&mut cx(&mut g), bear, CounterKind::Vow, 1);
    assert_eq!(barred(&g, &[bear], &[Target::Player(P0)]), vec![(bear, Target::Player(P0))]);
    let before = g.state.clone();
    let mark = g.state.mark();
    cantrip_api::instruction(&mut cx(&mut g), |c| {
        c.remove_counters(bear, CounterKind::Vow, 1);
        c.add_counters(bear, CounterKind::Vow, 1);
    });
    assert_eq!(g.state.obj(bear).counters(CounterKind::Vow), 1);
    assert!(barred(&g, &[bear], &[Target::Player(P0)]).is_empty());
    assert!(g.state.effects.is_empty(), "ended, not dormant");
    let after = g.state.clone();
    let (undos, redos) = g.state.step_back(mark);
    assert!(g.state == before, "exact counter/effect journal restoration");
    g.state.step_forward(undos, redos);
    assert!(g.state == after, "exact counter/effect redo");
    g.state.step_back(mark);
    restrict(&mut g, source, bear, P2, true);
    assert_eq!(barred(&g, &[bear], &[Target::Player(P0), Target::Player(P2)]), vec![(bear, Target::Player(P0)), (bear, Target::Player(P2))]);
}
#[test]
fn a_duration_that_never_started_cannot_restart() {
    let mut g = game();
    let source = put(&mut g, &OPEN, P0);
    let bear = put(&mut g, &BEAR, P1);
    restrict(&mut g, source, bear, P0, true);
    assert!(g.state.effects.is_empty());
    counter(&mut g, bear, 1);
    assert!(barred(&g, &[bear], &[Target::Player(P0)]).is_empty());
}
#[test]
fn moving_the_vow_never_restricts_the_recipient() {
    let mut g = game();
    let source = put(&mut g, &OPEN, P0);
    let bear = put(&mut g, &BEAR, P1);
    let other = put(&mut g, &BEAR, P1);
    counter(&mut g, bear, 1);
    restrict(&mut g, source, bear, P0, true);
    assert_eq!(crate::counters::mv(&mut cx(&mut g), bear, other, CounterKind::Vow, 1), 1);
    assert_eq!(g.state.obj(bear).counters(CounterKind::Vow), 0);
    assert_eq!(g.state.obj(other).counters(CounterKind::Vow), 1);
    assert!(barred(&g, &[bear, other], &[Target::Player(P0)]).is_empty());
}
#[test]
fn a_returned_creature_is_new_even_with_a_new_vow() {
    let mut g = game();
    let source = put(&mut g, &OPEN, P0);
    let bear = put(&mut g, &BEAR, P1);
    counter(&mut g, bear, 1);
    restrict(&mut g, source, bear, P0, true);
    let card = cantrip_api::Game::exile_all(&mut cx(&mut g), &[bear])[0];
    let returned = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), card, P1, false).unwrap();
    assert_ne!(returned, bear);
    counter(&mut g, returned, 1);
    assert!(barred(&g, &[returned], &[Target::Player(P0)]).is_empty());
}
#[test]
fn canonical_open_set_reaches_new_creatures_and_only_the_player() {
    let mut g = game();
    let source = put(&mut g, &OPEN, P0);
    let first = put(&mut g, &BEAR, P1);
    let walker = put(&mut g, &WALKER, P0);
    let timestamp = g.state.timestamp();
    let card = g.cards.id_of(&OPEN);
    g.state.effect_push(ContinuousEffect {
        source,
        controller: P0,
        timestamp,
        affected: source,
        kind: EffectKind::LastingRule { card, index: 0, about: Target::Player(P0) },
        duration: Duration::UntilNextTurnOf(P0),
        words: Words::NONE,
    });
    cantrip_api::Game::exile_all(&mut cx(&mut g), &[source]);
    let second = put(&mut g, &BEAR, P2);
    let targets = [Target::Player(P0), Target::Player(P2), Target::Object(walker)];
    assert_eq!(barred(&g, &[first, second], &targets), vec![(first, targets[0]), (second, targets[0])]);
    assert!(!crate::static_rules::attack_rules_allow_any(&g, g.cards, first, || vec![targets[0]]));
    assert!(crate::static_rules::attack_rules_allow_any(&g, g.cards, first, || vec![targets[2]]));
}
