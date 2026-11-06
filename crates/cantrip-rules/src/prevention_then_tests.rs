//! Prevention effects a spell or ability made that do more than prevent (CR 615.5), and "the next time ... would deal
//! damage, prevent that damage" (CR 615.8), over test cards: the rest of the effect happens once the damage event has been
//! dealt and is told what was prevented; the "next time" is used up by the one event; neither applies to damage that can't
//! be prevented (CR 615.12); both are ordered with the other effects on the damage (CR 616.1); and the source chosen is
//! asked of as it last existed (CR 608.2h, 609.7a).

use crate::ops::{damage, damage_together, exile_all, move_to};
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::prevention;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static BEAR: CardDef = CardDef {
    name: "Prevention Then Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// "[The effect's source] deals that much damage to that source's controller" (Deflecting Palm): the chosen source's controller
/// as it last existed (CR 608.2h).
fn deflect(g: &mut dyn cantrip_api::Game, asker: &Asker, chosen: ObjectId, prevented: &[Prevented]) {
    let to = g.controller(chosen);
    let amount = u32::try_from(Prevented::total(prevented)).expect("test damage is small");
    g.damage(asker.source, Target::Player(to), amount);
}

/// "[The effect's source] deals that much damage to [the other player]" (Refraction Trap's "any target", as a fixed one).
fn to_them(g: &mut dyn cantrip_api::Game, asker: &Asker, _: ObjectId, prevented: &[Prevented]) {
    let amount = u32::try_from(Prevented::total(prevented)).expect("test damage is small");
    g.damage(asker.source, Target::Player(THEM), amount);
}

/// The parts it was told of, as life its controller gains: one for each part, so a test can count them.
fn count_parts(g: &mut dyn cantrip_api::Game, asker: &Asker, _: ObjectId, prevented: &[Prevented]) {
    g.gain_life(asker.controller, prevented.len().min(i32::MAX as usize) as i32);
}

static PALM: CardDef = CardDef {
    name: "Prevention Then Test Palm",
    types: TypeSet::of(&[CardType::Instant]),
    granted_preventions: &[
        GrantedPreventionDef {
            then: Some(deflect),
            text: "The next time a source of your choice would deal damage to you this turn, prevent that damage. If damage is prevented this way, Prevention Then Test Palm deals that much damage to that source's controller.",
            ..prevention::FROM_CHOSEN_SOURCE
        },
        GrantedPreventionDef { then: Some(to_them), ..prevention::ANY_DAMAGE },
        prevention::ANY_DAMAGE,
        GrantedPreventionDef { then: Some(count_parts), ..prevention::FROM_CHOSEN_SOURCE },
        prevention::FROM_CHOSEN_SOURCE,
    ],
    granted_replacements: &[
        granted_replacement(ReplacementKind::Damage, never_applies, leave_it, "a replacement about its object"),
        granted_replacement(ReplacementKind::Damage, never_applies, leave_it, "deals that much damage instead")
            .text("Prevention Then Test Palm deals that much damage instead."),
    ],
    delayed: &[
        delayed_trigger(nothing, "Return that card to the battlefield at the beginning of the next end step."),
        delayed_trigger(nothing, "At the beginning of the next end step, Prevention Then Test Palm deals 2 damage to each opponent."),
    ],
    ..CardDef::BLANK
};

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn never_applies(_: &dyn Query, _: &Asker, _: ObjectId, _: &Proposal) -> bool {
    false
}

fn leave_it(_: &dyn Query, _: &Asker, _: ObjectId, _: &mut Proposal) {}

/// Its replacement definitions, by index in `granted_replacements`: one about an object, one whose text names the Palm.
const REPLACE_ABOUT: u8 = 0;
const REPLACE_NAMING: u8 = 1;

/// Its delayed abilities, by index in `delayed`: one about what it returns, one naming the Palm itself.
const ABOUT_A_CARD: u8 = 0;
const NAMING_ITSELF: u8 = 1;

/// Its definitions, by index in `granted_preventions`.
const DEFLECT: u8 = 0;
const REFRACT: u8 = 1;
const PLAIN: u8 = 2;
const COUNT: u8 = 3;
const PLAIN_FROM_CHOSEN: u8 = 4;

struct Table {
    game: Game,
    /// The resolved spell whose effects these are, in its owner's graveyard.
    palm: ObjectId,
    /// A creature THEM controls, the source chosen.
    bear: ObjectId,
    /// Another creature THEM controls, a source no effect watches.
    other: ObjectId,
    /// A creature YOU control.
    mine: ObjectId,
}

fn table() -> Table {
    let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR, &PALM])));
    let mut game = Game::new(cards, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    // Turn 1: no turn of the game is one in which damage can't be prevented (`no_prevention_turn` starts at 0).
    game.state.update_turn(|t| t.number = 1);
    let put = |game: &mut Game, card: &'static CardDef, owner: PlayerId, zone: Zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(card), owner, zone));
        game.state.zone_push(zone, id);
        id
    };
    let palm = put(&mut game, &PALM, YOU, Zone::graveyard(YOU));
    let bear = put(&mut game, &BEAR, THEM, Zone::BATTLEFIELD);
    let other = put(&mut game, &BEAR, THEM, Zone::BATTLEFIELD);
    let mine = put(&mut game, &BEAR, YOU, Zone::BATTLEFIELD);
    Table { game, palm, bear, other, mine }
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// A frame to ask questions in, its answers given in advance: what the run reads back as it asks (`Cx::ask_core`).
fn answered(cx: &mut Cx, answers: &[Answer]) {
    let frame = cx.s().agenda.len();
    cx.st().frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::BeginStep));
    for answer in answers {
        cx.st().update_frame(frame, |f| f.answers.push((None, answer.clone())));
    }
    cx.frame = frame;
}

/// The effect of the Palm's `index`th definition about `affected`, of damage to `to`, until end of turn.
fn shield(cx: &mut Cx, palm: ObjectId, affected: ObjectId, index: u8, to: Target, shield: Shield) {
    let card = cx.s().obj(palm).card;
    cantrip_api::Game::add_effect(cx, palm, affected, EffectKind::Prevention { card, index, to, shield }, Duration::EndOfTurn);
}

fn prevention_effects(cx: &Cx) -> Vec<Shield> {
    cx.s().effects.iter().filter_map(|e| match e.kind {
        EffectKind::Prevention { shield, .. } => Some(shield),
        _ => None,
    }).collect()
}

/// The damage events in the history, in order: source, recipient, amount.
fn dealt(cx: &Cx) -> Vec<(ObjectId, Target, u32)> {
    cx.g.log.iter().filter_map(|r| match r.event {
        Event::Damage { source, target, amount, .. } => Some((source, target, amount)),
        _ => None,
    }).collect()
}

fn prevented(cx: &Cx) -> Vec<(ObjectId, Target, u32)> {
    cx.g.log.iter().filter_map(|r| match r.event {
        Event::DamagePrevented { source, target, amount, .. } => Some((source, target, amount)),
        _ => None,
    }).collect()
}

/// CR 615.8, 615.5: the next time the chosen source would deal damage to you, all of it is prevented and the effect is used
/// up; then the Palm deals that much damage to the source's controller. The next damage from it is dealt normally, and
/// damage from a source the effect does not watch was never its to prevent.
#[test]
fn the_next_time_is_used_up_by_one_event_and_its_rest_deals_that_much() {
    let Table { mut game, palm, bear, other, .. } = table();
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, bear, DEFLECT, Target::Player(YOU), Shield::NextTime);
    assert_eq!(damage(&mut cx, other, Target::Player(YOU), 1, false), 1, "another source's damage is dealt");
    assert_eq!(prevention_effects(&cx), [Shield::NextTime], "and does not use the effect up");
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, true), 0);
    assert_eq!(cx.s().player(YOU).life, 19);
    assert_eq!(cx.s().player(THEM).life, 17, "the Palm deals the 3 prevented to the source's controller");
    assert_eq!(prevented(&cx), [(bear, Target::Player(YOU), 3)]);
    assert_eq!(dealt(&cx), [(other, Target::Player(YOU), 1), (palm, Target::Player(THEM), 3)], "dealt by the Palm, after the prevention");
    assert!(prevention_effects(&cx).is_empty(), "used up");
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 2, false), 2, "the next damage from it is dealt normally");
    assert_eq!(cx.s().player(YOU).life, 17);
    assert_eq!(cx.s().player(THEM).life, 17);
}

/// CR 615.5: the rest of a shield's effect is told how much it prevented, not how much would have been dealt, and happens
/// after the damage event - the 1 the shield left is dealt first.
#[test]
fn the_rest_of_a_shield_gets_the_amount_prevented_after_the_event() {
    let Table { mut game, palm, bear, .. } = table();
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, palm, REFRACT, Target::Player(YOU), Shield::Next(2));
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 1);
    assert_eq!(cx.s().player(YOU).life, 19);
    assert_eq!(cx.s().player(THEM).life, 18, "2 prevented, 2 dealt to them");
    assert_eq!(dealt(&cx), [(bear, Target::Player(YOU), 1), (palm, Target::Player(THEM), 2)]);
    assert!(prevention_effects(&cx).is_empty(), "the shield is used up");
}

/// CR 510.2, 615.5: combat damage is one event too - the rest of a shield's effect runs once it has been dealt, in the
/// combat damage step, before anything else happens.
#[test]
fn the_rest_of_a_shield_runs_after_combat_damage() {
    let Table { mut game, palm, bear, .. } = table();
    game.state.update_turn(|t| t.active = THEM);
    game.state.set_attackers(vec![Attacker::new(bear, Target::Player(YOU), YOU, TypeSet::NONE)]);
    game.state.update_turn(|t| t.set_step(Step::CombatDamage));
    let frame = game.state.agenda.len();
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::BeginStep));
    let mut cx = cx(&mut game);
    cx.frame = frame;
    shield(&mut cx, palm, palm, REFRACT, Target::Player(YOU), Shield::Next(2));
    crate::turn::begin_step(&mut cx).expect("no questions");
    assert_eq!(cx.s().player(YOU).life, 19, "the 1 the shield left");
    assert_eq!(cx.s().player(THEM).life, 18, "the 2 prevented, dealt to them");
    assert_eq!(dealt(&cx), [(bear, Target::Player(YOU), 1), (palm, Target::Player(THEM), 2)]);
}

/// CR 615.8, 120.2: "the next time [the source] would deal damage" (of damage to anyone) is one damage event however many
/// recipients it has: every part of it is prevented, the rest of the effect runs once and is told of each part, and the
/// effect is used up after it - not after the first part.
#[test]
fn the_next_time_prevents_every_part_of_one_simultaneous_event() {
    let Table { mut game, palm, bear, mine, .. } = table();
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, bear, COUNT, Target::None, Shield::NextTime);
    let hits = [(bear, Target::Player(YOU), 2), (bear, Target::Object(mine), 2)];
    assert_eq!(damage_together(&mut cx, &hits), [0, 0]);
    assert_eq!(cx.s().obj(mine).damage, 0);
    assert_eq!(cx.s().player(YOU).life, 22, "told of both parts, once");
    assert!(prevention_effects(&cx).is_empty());
    assert_eq!(damage_together(&mut cx, &hits), [2, 2], "the next event is dealt");
}

/// CR 615.12: damage that can't be prevented isn't, the rest of the effect ("if damage is prevented this way") does
/// nothing, and the effect is not used up by it (CR 609.7b).
#[test]
fn damage_that_cant_be_prevented_leaves_the_effect_and_does_nothing_more() {
    let Table { mut game, palm, bear, .. } = table();
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, bear, DEFLECT, Target::Player(YOU), Shield::NextTime);
    cantrip_api::Game::damage_cant_be_prevented(&mut cx);
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 3);
    assert_eq!(cx.s().player(YOU).life, 17);
    assert_eq!(cx.s().player(THEM).life, 20);
    assert!(prevented(&cx).is_empty());
    assert_eq!(prevention_effects(&cx), [Shield::NextTime]);
}

/// CR 616.1: a "next time" with more to do is ordered with a shield on the same damage by the player dealt it, and the
/// order changes what happens. The shield first prevents 2 and the Palm the 1 left, which it deals; the Palm first prevents
/// all 3, which it deals, and the shield is untouched.
#[test]
fn the_player_dealt_the_damage_orders_it_with_a_shield() {
    for (first, them, left) in [(1, 19, Vec::new()), (0, 17, vec![Shield::Next(2)])] {
        let Table { mut game, palm, bear, .. } = table();
        let mut cx = cx(&mut game);
        shield(&mut cx, palm, bear, DEFLECT, Target::Player(YOU), Shield::NextTime);
        shield(&mut cx, palm, palm, PLAIN, Target::Player(YOU), Shield::Next(2));
        answered(&mut cx, &[Answer::Index(first)]);
        assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 0);
        assert_eq!(cx.cursor, 1, "the order was asked");
        assert_eq!(cx.s().player(YOU).life, 20);
        assert_eq!(cx.s().player(THEM).life, them);
        assert_eq!(prevention_effects(&cx), left);
    }
}

/// CR 616.1f, 609.7b: another effect applied first prevents all the damage; what is left is no damage the "next time"
/// applies to (0 damage is no event), so it prevents nothing, does nothing more, and is not used up.
#[test]
fn damage_another_effect_prevented_in_full_leaves_the_next_time_waiting() {
    let Table { mut game, palm, bear, .. } = table();
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, bear, DEFLECT, Target::Player(YOU), Shield::NextTime);
    shield(&mut cx, palm, palm, PLAIN, Target::Player(YOU), Shield::All);
    answered(&mut cx, &[Answer::Index(1)]);
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 0);
    assert_eq!(cx.cursor, 1, "the order was asked: which applies first matters");
    assert_eq!(cx.s().player(THEM).life, 20);
    assert_eq!(prevented(&cx).len(), 1);
    assert_eq!(prevention_effects(&cx), [Shield::NextTime, Shield::All]);
}

/// CR 608.2h: a source that has left the battlefield deals its damage as it last existed (a dies trigger's), and "that
/// source's controller" is its controller then - not its owner, whose exile it is in now.
#[test]
fn a_source_that_has_left_is_asked_of_as_it_last_existed() {
    let Table { mut game, palm, bear, .. } = table();
    game.state.update_obj(bear, |o| o.owner = YOU);
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, bear, DEFLECT, Target::Player(YOU), Shield::NextTime);
    assert_eq!(exile_all(&mut cx, &[bear]).len(), 1);
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 0);
    assert_eq!(cx.s().player(YOU).life, 20);
    assert_eq!(cx.s().player(THEM).life, 17, "its last controller is dealt the damage");
}

/// CR 609.7a: a permanent spell chosen as the source is the permanent it becomes as it resolves, too; once that permanent
/// has left, the new object is not it (CR 400.7).
#[test]
fn a_chosen_permanent_spell_is_the_permanent_it_becomes() {
    let Table { mut game, palm, .. } = table();
    let card = game.cards.id_of(&BEAR);
    let spell = game.state.push_obj(Obj::new(card, THEM, Zone::STACK));
    game.state.zone_push(Zone::STACK, spell);
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, spell, DEFLECT, Target::Player(YOU), Shield::NextTime);
    let permanent = move_to(&mut cx, spell, Zone::BATTLEFIELD);
    assert_ne!(permanent, spell);
    assert_eq!(damage(&mut cx, permanent, Target::Player(YOU), 3, true), 0);
    assert_eq!(cx.s().player(THEM).life, 17);
    // "Any damage dealt by the permanent that spell becomes": once it has left too, as it last existed (Perilous Myr dying).
    shield(&mut cx, palm, spell, DEFLECT, Target::Player(YOU), Shield::NextTime);
    let card = exile_all(&mut cx, &[permanent])[0];
    assert_eq!(damage(&mut cx, permanent, Target::Player(YOU), 3, false), 0, "the permanent, after it has left");
    assert_eq!(cx.s().player(THEM).life, 14);
    shield(&mut cx, palm, spell, DEFLECT, Target::Player(YOU), Shield::NextTime);
    let again = move_to(&mut cx, card, Zone::BATTLEFIELD);
    assert_eq!(damage(&mut cx, again, Target::Player(YOU), 2, false), 2, "a new object after that");
}

/// CR 609.7a: the sources a player may choose - permanents, spells, objects an object on the stack refers to even after they
/// have left, objects a waiting prevention effect refers to - and "the next time a source of your choice would deal damage
/// to you" asks for one as it resolves and watches it. A resolved spell whose waiting effect never names it (Fog's "prevent
/// all combat damage") is no object that effect refers to; one whose effect does ("Deflecting Palm deals that much damage")
/// is.
#[test]
fn a_source_of_your_choice_is_chosen_as_the_effect_resolves() {
    let Table { mut game, palm, bear, other, mine } = table();
    let cards = game.cards;
    let spell = game.state.push_obj(Obj::new(cards.id_of(&PALM), YOU, Zone::STACK));
    game.state.zone_push(Zone::STACK, spell);
    let named = game.state.push_obj(Obj::new(cards.id_of(&PALM), YOU, Zone::graveyard(YOU)));
    game.state.zone_push(Zone::graveyard(YOU), named);
    let mut cx = cx(&mut game);
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(cards.id_of(&PALM)), YOU, &[]);
    cx.st().stack_push(item);
    shield(&mut cx, palm, mine, PLAIN, Target::Object(mine), Shield::All);
    shield(&mut cx, palm, palm, PLAIN, Target::None, Shield::All);
    assert_eq!(prevention::damage_sources(&cx), [bear, other, mine, spell], "effects that never name the Palm");
    shield(&mut cx, named, bear, DEFLECT, Target::Player(YOU), Shield::NextTime);
    assert_eq!(prevention::damage_sources(&cx), [bear, other, mine, spell, named], "one that names its source");
    cx.st().effects_retain(|e| e.source != palm);
    answered(&mut cx, &[Answer::Cards(vec![other])]);
    let item = cx.s().stack[0];
    assert!(matches!(prevention::next_time_chosen_source_would_damage_you(&mut cx, &item, DEFLECT), Ok(Some(o)) if o == other));
    assert_eq!(damage(&mut cx, other, Target::Player(YOU), 2, false), 0);
    assert_eq!(cx.s().player(THEM).life, 18, "dealt by the spell");
}

/// CR 615.8, 609.7b: a "next time" that does nothing more is used up by the one event all the same, and damage it prevents
/// none of (another source's) leaves it waiting.
#[test]
fn a_next_time_with_nothing_more_to_do_is_used_up_by_one_event() {
    let Table { mut game, palm, bear, other, .. } = table();
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, bear, PLAIN_FROM_CHOSEN, Target::Player(YOU), Shield::NextTime);
    assert_eq!(damage(&mut cx, other, Target::Player(YOU), 1, false), 1);
    assert_eq!(prevention_effects(&cx), [Shield::NextTime], "another source's damage leaves it waiting");
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 0);
    assert!(prevention_effects(&cx).is_empty(), "used up by the event it prevented");
    assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 2, false), 2, "the next damage from it is dealt");
    assert_eq!(cx.s().player(YOU).life, 17);
}

/// CR 616.1, 615.5: an effect that prevents all the damage but has more to do is not one whose order changes nothing - the
/// player dealt the damage orders it with another that prevents all of it. The one with more to do first prevents the 3,
/// which it deals to them; the other first leaves it nothing to prevent.
#[test]
fn prevent_all_with_more_to_do_is_ordered_with_another_prevent_all() {
    for (first, them) in [(0, 17), (1, 20)] {
        let Table { mut game, palm, bear, .. } = table();
        let mut cx = cx(&mut game);
        shield(&mut cx, palm, palm, REFRACT, Target::Player(YOU), Shield::All);
        shield(&mut cx, palm, palm, PLAIN, Target::Player(YOU), Shield::All);
        answered(&mut cx, &[Answer::Index(first)]);
        assert_eq!(damage(&mut cx, bear, Target::Player(YOU), 3, false), 0);
        assert_eq!(cx.cursor, 1, "the order was asked");
        assert_eq!(cx.s().player(YOU).life, 20);
        assert_eq!(cx.s().player(THEM).life, them);
    }
}

/// CR 609.7a, 400.7: only a chosen permanent spell is followed to the permanent it becomes; a chosen card that is put onto
/// the battlefield from a graveyard is a new object, and its damage is not the chosen source's.
#[test]
fn a_chosen_card_put_onto_the_battlefield_is_a_new_object() {
    let Table { mut game, palm, .. } = table();
    let card = game.cards.id_of(&BEAR);
    let dead = game.state.push_obj(Obj::new(card, THEM, Zone::graveyard(THEM)));
    game.state.zone_push(Zone::graveyard(THEM), dead);
    let mut cx = cx(&mut game);
    shield(&mut cx, palm, dead, DEFLECT, Target::Player(YOU), Shield::NextTime);
    let returned = move_to(&mut cx, dead, Zone::BATTLEFIELD);
    assert_ne!(returned, dead);
    assert_eq!(damage(&mut cx, returned, Target::Player(YOU), 3, false), 3);
    assert_eq!(cx.s().player(THEM).life, 20);
}

/// CR 609.7a: the sources that have left the zone they were in and are still choosable - one an object on the stack
/// targets, one a waiting prevention effect is about, one a waiting delayed triggered ability is about - and the command
/// zone's face-up objects, not its face-down ones.
#[test]
fn a_source_of_your_choice_may_be_an_object_that_has_left() {
    let Table { mut game, palm, .. } = table();
    let cards = game.cards;
    let put = |game: &mut Game, zone: Zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(&BEAR), THEM, zone));
        game.state.zone_push(zone, id);
        id
    };
    let (targeted, about, later) = (put(&mut game, Zone::BATTLEFIELD), put(&mut game, Zone::BATTLEFIELD), put(&mut game, Zone::BATTLEFIELD));
    let face_up = put(&mut game, Zone::COMMAND);
    let face_down = put(&mut game, Zone::COMMAND);
    game.state.update_obj(face_down, |o| o.face_down = true);
    let spell = game.state.push_obj(Obj::new(cards.id_of(&PALM), YOU, Zone::STACK));
    game.state.zone_push(Zone::STACK, spell);
    let mut cx = cx(&mut game);
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(cards.id_of(&PALM)), YOU, &[Target::Object(targeted)]);
    cx.st().stack_push(item);
    shield(&mut cx, palm, about, PLAIN, Target::Object(about), Shield::All);
    let step = DelayedWhen::Step { step: Step::End, on_turn_of: None };
    cantrip_api::Game::delayed_trigger(&mut cx, step, spell, YOU, AbilityRef::delayed(cards.id_of(&PALM), ABOUT_A_CARD), Target::Object(later));
    assert_eq!(exile_all(&mut cx, &[targeted, about, later]).len(), 3);
    let sources = prevention::damage_sources(&cx);
    for (id, what) in [(targeted, "targeted"), (about, "an effect is about"), (later, "a delayed trigger is about"), (face_up, "face up")] {
        assert!(sources.contains(&id), "{what}");
    }
    assert!(!sources.contains(&face_down), "a face-down object in the command zone is not choosable");
}

/// CR 609.7a, 603.7d: a waiting delayed triggered ability refers to its source - the resolved spell that made it, in a
/// graveyard now - only where its text names it ("At the beginning of the next end step, [this spell] deals 2 damage"); one
/// that says "return that card" refers to the card, not the spell.
#[test]
fn a_waiting_delayed_trigger_offers_its_source_only_where_it_names_it() {
    let Table { mut game, bear, .. } = table();
    let cards = game.cards;
    let resolved = |game: &mut Game| {
        let id = game.state.push_obj(Obj::new(cards.id_of(&PALM), YOU, Zone::graveyard(YOU)));
        game.state.zone_push(Zone::graveyard(YOU), id);
        id
    };
    let (silent, naming) = (resolved(&mut game), resolved(&mut game));
    let mut cx = cx(&mut game);
    let step = DelayedWhen::Step { step: Step::End, on_turn_of: None };
    let palm = cards.id_of(&PALM);
    cantrip_api::Game::delayed_trigger(&mut cx, step, silent, YOU, AbilityRef::delayed(palm, ABOUT_A_CARD), Target::Object(bear));
    cantrip_api::Game::delayed_trigger(&mut cx, step, naming, YOU, AbilityRef::delayed(palm, NAMING_ITSELF), Target::None);
    let sources = prevention::damage_sources(&cx);
    assert!(sources.contains(&bear), "what it is about");
    assert!(!sources.contains(&silent), "a source its text never names");
    assert!(sources.contains(&naming), "a source its text names");
}

/// CR 609.7a, 702.88a: the rules' own delayed abilities are read as a card's are. Rebound's "At the beginning of your next
/// upkeep, you may cast this card from exile" refers to the exiled card, which is offered while it waits; the untimely
/// cleanup sacrifice ("the controller of the permanent it became sacrifices it") refers to its permanent, not to the spell
/// that made it.
#[test]
fn the_rules_own_delayed_abilities_offer_their_source_only_where_they_name_it() {
    let Table { mut game, bear, .. } = table();
    let cards = game.cards;
    let put = |game: &mut Game, zone: Zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(&PALM), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let (rebounding, untimely) = (put(&mut game, Zone::exile(YOU)), put(&mut game, Zone::graveyard(YOU)));
    let waiting = |when, source, ability, subject| cantrip_state::Delayed {
        when,
        source,
        controller: YOU,
        ability,
        subject,
        members: cantrip_state::Span::default(),
        since: 0,
        acquired: Acquired::OWN,
        words: None,
    };
    let upkeep = DelayedWhen::Step { step: Step::Upkeep, on_turn_of: Some(YOU) };
    let cleanup = DelayedWhen::Step { step: Step::Cleanup, on_turn_of: None };
    let sacrifice = AbilityRef { card: cards.intrinsic, index: cantrip_api::intrinsic::UNTIMELY_SACRIFICE };
    game.state.delayed_push(waiting(upkeep, rebounding, AbilityRef::rebound(cards.id_of(&PALM)), Target::None));
    game.state.delayed_push(waiting(cleanup, untimely, sacrifice, Target::Object(bear)));
    let sources = prevention::damage_sources(&game);
    assert!(sources.contains(&rebounding), "the exiled card rebound's waiting ability names");
    assert!(!sources.contains(&untimely), "the spell whose untimely sacrifice waits, which it never names");
    assert!(sources.contains(&bear), "the permanent the untimely sacrifice is about");
}

/// CR 609.7a: a waiting replacement effect - a regeneration shield's included (CR 701.19a) - refers to the object it is about,
/// even one that has left; and to its source only where its text names it.
#[test]
fn a_waiting_replacement_offers_what_it_is_about_and_a_source_it_names() {
    let Table { mut game, .. } = table();
    let cards = game.cards;
    let put = |game: &mut Game, card: &'static CardDef, zone: Zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(card), THEM, zone));
        game.state.zone_push(zone, id);
        id
    };
    let (replaced, regenerating) = (put(&mut game, &BEAR, Zone::BATTLEFIELD), put(&mut game, &BEAR, Zone::BATTLEFIELD));
    let (silent, naming, shielding) =
        (put(&mut game, &PALM, Zone::graveyard(THEM)), put(&mut game, &PALM, Zone::graveyard(THEM)), put(&mut game, &PALM, Zone::graveyard(THEM)));
    let mut cx = cx(&mut game);
    let palm = cards.id_of(&PALM);
    cantrip_api::Game::add_effect(&mut cx, silent, replaced, EffectKind::Replacement { card: palm, index: REPLACE_ABOUT }, Duration::EndOfTurn);
    cantrip_api::Game::add_effect(&mut cx, naming, replaced, EffectKind::Replacement { card: palm, index: REPLACE_NAMING }, Duration::EndOfTurn);
    cantrip_api::Game::add_effect(&mut cx, shielding, regenerating, EffectKind::RegenerationShield, Duration::EndOfTurn);
    assert_eq!(exile_all(&mut cx, &[replaced, regenerating]).len(), 2);
    let sources = prevention::damage_sources(&cx);
    for (id, what) in [(replaced, "a replacement is about"), (regenerating, "a regeneration shield is about"), (naming, "a source its text names")] {
        assert!(sources.contains(&id), "{what}");
    }
    for (id, what) in [(silent, "a source its text never names"), (shielding, "a regeneration shield's source")] {
        assert!(!sources.contains(&id), "{what}");
    }
}
