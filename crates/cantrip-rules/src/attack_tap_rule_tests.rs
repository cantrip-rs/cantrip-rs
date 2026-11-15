//! "This creature can't attack" granted as an ability (`Keyword::CantAttack`, CR 508.1c, 113.1a) and a resolved rule that
//! attacking doesn't cause a set of creatures to tap (`LastingRuleKind::AttackingDoesntCauseTap`, CR 508.1f, 611.2c), over
//! test cards, through a real declaration of attackers.

use crate::{Game, Setup};
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Attack Tap Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static BEAR: CardDef =
    CardDef { name: "Attack Tap Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static GOLEM: CardDef = CardDef {
    name: "Attack Tap Test Golem",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// Nonartifact creatures: the Bear and not the Golem.
fn nonartifact_creatures(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && !q.chars(id).is(CardType::Artifact)
}

static RULE: CardDef = CardDef {
    name: "Attack Tap Test Rule",
    types: TypeSet::of(&[CardType::Sorcery]),
    lasting_rules: &[LastingRule {
        text: "Attacking doesn't cause nonartifact creatures to tap this turn.",
        rule: LastingRuleKind::AttackingDoesntCauseTap,
        applies: nonartifact_creatures,
    }],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &GOLEM, &RULE])));
    let deck = vec![table.id_of(&FILLER); 30];
    let mut g = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    g.verify = true;
    g
}

fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), P0, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn effect(g: &mut Game, source: ObjectId, affected: ObjectId, kind: EffectKind) {
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source, controller: P0, timestamp, affected, kind, duration: Duration::EndOfTurn, words: Words::NONE });
}

/// Plays P0's first turn to its declaration of attackers, declares every eligible creature attacking P1, and returns which
/// were eligible; the declaration is then made.
fn attack_with_everything(g: &mut Game) -> Vec<ObjectId> {
    for _ in 0..200 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { eligible, .. } => {
                let eligible = eligible.clone();
                g.answer(request.id, request.player, Answer::Attack(eligible.iter().map(|&id| (id, Target::Player(P1))).collect())).unwrap();
                g.run();
                return eligible;
            }
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    panic!("no declaration of attackers");
}

/// CR 508.1f, 611.2c: the rule's set - the Bear, not the Golem - attacks without becoming tapped, though it has no vigilance
/// and has lost all its abilities; the Golem becomes tapped as usual.
#[test]
fn a_resolved_rule_keeps_the_creatures_it_takes_in_untapped_as_they_attack() {
    let mut g = game();
    let bear = put(&mut g, &BEAR);
    let golem = put(&mut g, &GOLEM);
    let card = g.cards.id_of(&RULE);
    effect(&mut g, golem, golem, EffectKind::LastingRule { card, index: 0, about: Target::None });
    effect(&mut g, golem, bear, EffectKind::LoseAllAbilities);
    assert_eq!(attack_with_everything(&mut g), vec![bear, golem]);
    assert!(!g.state.obj(bear).tapped, "the rule is no ability, so losing them all keeps it");
    assert!(!g.chars(bear).has(Keyword::Vigilance));
    assert!(g.state.obj(golem).tapped);
}

/// CR 508.1c, 113.1a, 613.1f: a creature that gained "this creature can't attack" may not be declared; one that then loses all
/// its abilities (a later timestamp) has it no more and may.
#[test]
fn a_gained_cant_attack_bars_the_creature_until_it_loses_its_abilities() {
    let mut g = game();
    let bear = put(&mut g, &BEAR);
    let golem = put(&mut g, &GOLEM);
    effect(&mut g, bear, bear, EffectKind::AddKeywords(Keywords::of(&[Keyword::CantAttack])));
    assert_eq!(attack_with_everything(&mut g), vec![golem]);
    let mut g = game();
    let bear = put(&mut g, &BEAR);
    let golem = put(&mut g, &GOLEM);
    effect(&mut g, bear, bear, EffectKind::AddKeywords(Keywords::of(&[Keyword::CantAttack])));
    effect(&mut g, golem, bear, EffectKind::LoseAllAbilities);
    assert_eq!(attack_with_everything(&mut g), vec![bear, golem]);
    assert!(g.state.obj(bear).tapped);
}
