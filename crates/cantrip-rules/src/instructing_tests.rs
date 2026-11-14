//! The spell or ability whose instructions are being carried out (`Query::instructing`, CR 608.2c), whose effect the damage
//! they have a source deal is (CR 120.2b): the one resolving, but not while a mana ability resolves in the middle of it
//! (CR 605.3b), nor while a prevention or replacement effect does what it does besides changing the event (CR 614.1a, 615.5).
//! Over test cards: a spell of YOU's that targets THEM's creature, resolving; "if a spell or ability that targets that creature
//! would cause a source to deal damage to that creature, prevent that damage" (Silhouette's kind) made about the creature; and
//! "prevent the next damage that would be dealt to you; if damage is prevented this way, [this] deals that much damage to
//! [that creature]" (Comeuppance's kind).

use crate::actions::{stack_item, WithWords};
use crate::{Cx, Game, Setup, StateCheck};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static BEAR: CardDef =
    CardDef { name: "Instructing Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(5), ..CardDef::BLANK };

/// The spell resolving, of YOU's, that targets the creature.
static SPELL: CardDef = CardDef { name: "Instructing Spell", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };

/// Damage to the creature the effect is about, while a spell or ability that targets that creature is what deals it.
fn by_one_targeting_it(q: &dyn Query, _: &Asker, creature: ObjectId, d: &DamageEvent) -> bool {
    d.target == Target::Object(creature) && q.instructing().is_some_and(|item| q.state().targets(item).contains(&Target::Object(creature)))
}

fn any(_: &dyn Query, _: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    true
}

/// "[This] deals that much damage to [that creature]": the rest of the prevention effect (CR 615.5), its own.
fn strike_back(g: &mut dyn cantrip_api::Game, asker: &Asker, creature: ObjectId, parts: &[Prevented]) {
    g.damage(asker.source, Target::Object(creature), Prevented::total(parts));
}

static SHIELDS: CardDef = CardDef {
    name: "Instructing Shields",
    types: TypeSet::of(&[CardType::Instant]),
    granted_preventions: &[
        GrantedPreventionDef { hint: "prevent damage by what targets it", damage: "damage", of_it: None, applies: by_one_targeting_it, portion: None, then: None, gain_prevented: false, text: "" },
        GrantedPreventionDef { hint: "prevent and strike back", damage: "damage", of_it: None, applies: any, portion: None, then: Some(strike_back), gain_prevented: false, text: "" },
    ],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR, &SPELL, &SHIELDS])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    // Turn 1, where damage can be prevented at all.
    g.state.update_turn(|t| t.number = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, controller: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, zone));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

/// YOU's spell targeting `targets`, put on the stack and resolving: the frame its card code runs in.
fn resolving(g: &mut Game, targets: &[Target]) -> (usize, ObjectId) {
    let card = g.cards.id_of(&SPELL);
    let spell = g.state.push_obj(Obj::new(card, YOU, Zone::STACK));
    g.state.zone_push(Zone::STACK, spell);
    let item = stack_item(StackKind::Spell, spell, AbilityRef { card, index: 0 }, YOU, targets);
    g.state.stack_push(item);
    let serial = g.state.stack.last().expect("the spell").serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    (g.state.agenda.len() - 1, spell)
}

fn cx(g: &mut Game, frame: usize) -> Cx<'_> {
    let cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    let resolving = cx.resolving().map(|item| item.object);
    cx.g.check = StateCheck { depth: 1, resolving, ..StateCheck::NONE };
    cx
}

/// A shield of `SHIELDS`' `index`th prevention, made by a resolved card of `controller`'s about `affected`, of damage `to`.
fn shield(cx: &mut Cx, controller: PlayerId, index: u8, affected: ObjectId, to: Target) {
    let source = put(cx.g, &SHIELDS, controller, Zone::graveyard(controller));
    let card = cx.g.cards.id_of(&SHIELDS);
    cantrip_api::Game::add_effect(cx, source, affected, EffectKind::Prevention { card, index, to, shield: Shield::All }, Duration::EndOfTurn);
}

/// The resolving spell is what its instructions are while they are carried out, and through the words a waiting effect is read
/// with (`WithWords`); not outside its resolution, nor while a mana ability resolves in it.
#[test]
fn the_resolving_spell_is_instructing_only_while_its_own_instructions_are_carried_out() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let (frame, spell) = resolving(&mut g, &[Target::Object(bear)]);
    let mut cx = cx(&mut g, frame);
    assert_eq!(Query::instructing(&cx).map(|i| i.object), Some(spell));
    let words = WithWords { q: &cx, source: spell, words: Query::words(&cx, spell), into: Target::None, value: 0 };
    assert_eq!(words.instructing().map(|i| i.object), Some(spell));
    cx.mana_source = Some(bear);
    assert!(Query::instructing(&cx).is_none(), "a mana ability's instructions are its own (CR 605.3b)");
    cx.mana_source = None;
    cx.g.check.replacement_callback = true;
    assert!(Query::instructing(&cx).is_none(), "a replacement or prevention effect's further instructions are its own (CR 615.5)");
    let priority = cx.g.state.agenda.len() - 2;
    let outside = Cx { g: cx.g, frame: priority, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    assert!(Query::instructing(&outside).is_none(), "no spell or ability resolves with priority");
}

/// What a replacement effect or a mana ability does in the middle of a resolution is that effect's own (CR 609.1, 614.1a,
/// 605.3b): the discard, sacrifice or search it has a player make is caused by its controller (`Cx::effect_controller`, CR
/// 101.2), not by the controller of the spell resolving. Outside, it is the spell's controller's; with priority, nobody's.
#[test]
fn a_replacement_effects_own_instructions_are_caused_by_its_controller() {
    let mut g = game();
    let (frame, _) = resolving(&mut g, &[]);
    let mut cx = cx(&mut g, frame);
    assert_eq!(cx.effect_controller(), Some(YOU), "the spell's instructions are its controller's");
    assert_eq!(cx.acting_as(THEM, |cx| cx.effect_controller()), Some(THEM), "a replacement effect's are its controller's");
    assert_eq!(cx.effect_controller(), Some(YOU), "and the spell's again once it is done");
    let priority = cx.g.state.agenda.len() - 2;
    let outside = Cx { g: cx.g, frame: priority, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    assert_eq!(outside.effect_controller(), None, "a player's own action has no spell or ability causing it");
}

/// Damage the resolving spell that targets the creature has a source deal to it is prevented; the same damage dealt while no
/// spell or ability resolves is not.
#[test]
fn damage_the_resolving_spell_targeting_the_creature_deals_it_meets_an_effect_about_that() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let other = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let (frame, _) = resolving(&mut g, &[Target::Object(bear)]);
    let mut cx = cx(&mut g, frame);
    shield(&mut cx, THEM, 0, bear, Target::Object(bear));
    // The source is another creature (a fight's, CR 701.14): what causes the damage is the spell.
    assert_eq!(crate::ops::damage(&mut cx, other, Target::Object(bear), 2, false), 0);
    assert_eq!(cx.s().obj(bear).damage, 0);
    let priority = cx.g.state.agenda.len() - 2;
    let mut outside = Cx { g: cx.g, frame: priority, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    assert_eq!(crate::ops::damage(&mut outside, other, Target::Object(bear), 2, false), 2);
    assert_eq!(outside.s().obj(bear).damage, 2);
}

/// What a prevention effect does besides preventing (CR 615.5) is its own, not the resolving spell's, though that spell
/// targets the creature and its damage made the effect apply: Comeuppance's damage to a creature is not prevented by an
/// effect about damage that a spell or ability targeting that creature causes.
#[test]
fn a_prevention_effects_further_damage_is_not_the_resolving_spells() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let (frame, spell) = resolving(&mut g, &[Target::Object(bear)]);
    let mut cx = cx(&mut g, frame);
    shield(&mut cx, THEM, 0, bear, Target::Object(bear));
    cantrip_api::Game::end_sentence(&mut cx);
    shield(&mut cx, THEM, 1, bear, Target::Player(THEM));
    assert_eq!(crate::ops::damage(&mut cx, spell, Target::Player(THEM), 3, false), 0);
    assert_eq!(cx.s().player(THEM).life, 20);
    assert_eq!(cx.s().obj(bear).damage, 3, "the strike back is dealt");
}
