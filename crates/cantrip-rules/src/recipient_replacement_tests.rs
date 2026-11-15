//! A replacement effect a resolving ability made that names whom the damage it watches for is dealt to (`EffectKind::Replacement`'s
//! `to`), as Jade Monolith's "the next time a source of your choice would deal damage to target creature this turn, that source
//! deals that damage to you instead" does (CR 609.7a, 614.1a, 614.9): it is asked only about damage to that recipient, and it is
//! about the chosen source, which it outlives (CR 609.7a: "even if that object is no longer in the zone it used to be in").

use super::*;
use crate::Game as RulesGame;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static SOURCE: CardDef = CardDef {
    name: "Recipient replacement source probe",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

static CREATURE: CardDef = CardDef {
    name: "Recipient replacement creature probe",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(5),
    ..CardDef::BLANK
};

fn from_chosen(_: &dyn Query, _: &Asker, chosen: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.source == chosen)
}

fn to_controller(_: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.target = Target::Player(asker.controller);
    }
}

static MONOLITH: CardDef = CardDef {
    name: "Recipient replacement probe",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, from_chosen, to_controller, "deals that damage to you instead").once()],
    ..CardDef::BLANK
};

/// THEM's source, YOU's effect from the probe about it, of damage to THEM's creature: the game, the source and the creature.
fn game() -> (RulesGame, ObjectId, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &CREATURE, &MONOLITH])));
    let mut game = RulesGame::new(cards, &crate::Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let mut put = |def: &'static CardDef, owner: PlayerId| {
        let id = game.state.push_obj(Obj::new(cards.id_of(def), owner, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let (source, creature, monolith) = (put(&SOURCE, THEM), put(&CREATURE, THEM), put(&MONOLITH, YOU));
    game.state.effect_push(ContinuousEffect {
        source: monolith, affected: source, controller: YOU,
        kind: EffectKind::Replacement { card: cards.id_of(&MONOLITH), index: 0, to: Target::Object(creature) },
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE,
    });
    game.state.update_turn(|t| t.number = 1);
    (game, source, creature)
}

fn cx(game: &mut RulesGame) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn waiting(cx: &Cx) -> bool {
    cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::Replacement { .. }))
}

/// The chosen source's damage to another recipient is dealt there and leaves the effect waiting (CR 609.7b); its next damage to
/// the named creature is dealt to the effect's controller instead, and the effect is used up.
#[test]
fn only_damage_to_the_named_recipient_is_replaced() {
    let (mut game, source, creature) = game();
    let mut cx = cx(&mut game);
    assert_eq!(damage(&mut cx, source, Target::Player(THEM), 2, false), 2);
    assert_eq!(cx.s().player(THEM).life, 18);
    assert!(waiting(&cx), "damage to another recipient does not use it up");
    damage(&mut cx, source, Target::Object(creature), 2, false);
    assert_eq!((cx.s().player(YOU).life, cx.s().obj(creature).damage), (18, 0));
    assert!(!waiting(&cx));
}

/// Once the chosen source has left the battlefield the effect still waits for it: the damage it deals as it last existed to
/// the named creature is dealt to the effect's controller instead (CR 609.7a).
#[test]
fn the_effect_outlives_the_source_it_is_about() {
    let (mut game, source, creature) = game();
    let mut cx = cx(&mut game);
    assert_eq!(exile_all(&mut cx, &[source]).len(), 1);
    // What the state-based check of lasting effects keeps (`settle`): one about an object ends with it unless it outlives it.
    assert!(cx.s().effects.iter().all(|e| e.kind.outlives_affected()), "it is not ended with the source");
    damage(&mut cx, source, Target::Object(creature), 3, false);
    assert_eq!((cx.s().player(YOU).life, cx.s().obj(creature).damage), (17, 0));
}
