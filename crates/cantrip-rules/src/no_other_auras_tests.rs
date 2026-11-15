//! "Enchanted land has indestructible and can't be enchanted by other Auras." (Consecrate Land) and "Enchanted creature can't be
//! the target of spells and can't be enchanted by other Auras." (Anti-Magic Aura): `TargetingRuleKind::NoOtherAuras`, a rule of
//! the Aura's (CR 303.4: "Other effects can limit what a permanent can be enchanted by"). Another Aura spell may target the
//! permanent (CR 303.4a: its enchant ability alone defines its targets) and, resolving, is put into its owner's graveyard
//! instead of entering (CR 303.4i); another Aura can't enter attached to it (CR 303.4f, 303.4i) nor move onto it (CR 303.4j), and one
//! already attached is put into its owner's graveyard (CR 303.4c, 704.5m; Consecrate Land's 2006 ruling). The granting Aura
//! itself stays; a second copy is another Aura, so it is refused, and two attached at once remove each other. The rule follows
//! the Aura where it moves, ends as it leaves, and ends with the Aura's abilities (CR 613.1f) - not with the permanent's, whose
//! ability it is not. Over test cards shaped like Anti-Magic Aura.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::attach::ENCHANTED_NO_OTHER_AURAS;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static LAND: CardDef = CardDef { name: "Sole Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Sole Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static SORCERY: CardDef = CardDef { name: "Sole Sorcery", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

fn creature_on_the_battlefield(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// An "enchant creature" Aura with `rules`.
const fn aura(name: &'static str, rules: &'static [TargetingRule]) -> CardDef {
    CardDef {
        name,
        colors: Some(ColorSet::of(Color::Blue)),
        types: TypeSet::of(&[CardType::Enchantment]),
        subtypes: SubtypeSet::of(&[Subtype::Aura]),
        enchant: Some(target(creature_on_the_battlefield)),
        targeting_rules: rules,
        ..CardDef::BLANK
    }
}

/// "Enchanted creature can't be enchanted by other Auras."
static SOLE_AURA: CardDef = aura("Sole Aura", &[ENCHANTED_NO_OTHER_AURAS]);
static PLAIN_AURA: CardDef = aura("Plain Aura", &[]);

static EQUIPMENT: CardDef = CardDef {
    name: "Sole Equipment",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};

/// The Aura a blue creature with bestow is while it is bestowed (CR 702.103b): an Aura enchantment with "enchant creature".
static BESTOWED: CardDef = CardDef { name: "Sole Bestowed", power: None, toughness: None, ..aura("Sole Bestowed", &[]) };
/// A blue 2/2 creature with bestow.
static BESTOWER: CardDef = CardDef {
    name: "Sole Bestower",
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Enchantment, CardType::Creature]),
    bestow: Some(&BESTOWED),
    ..BEAR
};

/// A creature with protection from blue, the Plain Aura's color.
static WARDED_BEAR: CardDef = CardDef { name: "Sole Warded Bear", protection: ColorSet::of(Color::Blue), ..BEAR };

fn an_enchantment_spell_cast(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, .. } if q.chars(*spell).is(CardType::Enchantment) => Fired::PLAIN,
        _ => None,
    }
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Whenever a player casts an enchantment spell, do nothing."
static WATCHER: CardDef = CardDef {
    name: "Sole Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::SPELL_CAST, an_enchantment_spell_cast), nothing, "Whenever a player casts an enchantment spell, do nothing.")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &LAND,
        &BEAR,
        &SORCERY,
        &SOLE_AURA,
        &PLAIN_AURA,
        &EQUIPMENT,
        &WARDED_BEAR,
        &WATCHER,
        &BESTOWED,
        &BESTOWER,
    ])));
    let deck = || vec![table.id_of(&LAND); 20];
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
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

/// `def` on the battlefield, attached to `host` however it got there.
fn attached(g: &mut Game, def: &'static CardDef, host: ObjectId) -> ObjectId {
    let id = put(g, def, Zone::BATTLEFIELD);
    g.state.update_obj(id, |o| o.attached_to = Some(host));
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// The state-based actions performed in a settling frame of the game's own, as the rules perform them.
fn state_based_actions(g: &mut Game) {
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let frame = g.state.agenda.len() - 1;
    let mut cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    crate::settle::state_based_actions_alone(&mut cx).expect("nothing is asked");
}

fn stays(g: &Game, id: ObjectId, host: ObjectId) -> bool {
    g.state.obj(id).zone.is(ZoneKind::Battlefield) && g.state.obj(id).attached_to == Some(host)
}

fn went_to_the_graveyard(g: &Game, id: ObjectId) -> bool {
    g.state.obj(id).became.is_some_and(|new| g.state.obj(new).zone.is(ZoneKind::Graveyard))
}

/// Whether the spell `spell` on the stack could target `what` (CR 601.2c, 608.2b).
fn spell_can_target(g: &Game, spell: ObjectId, what: ObjectId) -> bool {
    let asker = Asker { instance: None, source: spell, controller: YOU, by: None, spell: true };
    crate::layers::can_target(g.cards, &g.state, &g.log, &asker, Target::Object(what))
}

/// Whether another Aura could be attached to `host` now: by an effect that attaches (CR 303.4j, `Game::can_attach`).
fn can_attach(g: &mut Game, aura: ObjectId, host: ObjectId) -> bool {
    cantrip_api::Game::can_attach(&cx(g), aura, host)
}

/// "Can't be enchanted by" limits what may enchant, not what may be targeted: an Aura spell's legal targets are its enchant
/// ability's alone (CR 303.4a, 115.1b, 702.5a), so another Aura spell - a second copy too - may target the enchanted creature.
#[test]
fn another_aura_spell_may_target_the_permanent() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SOLE_AURA, bear);
    let spell = put(&mut g, &PLAIN_AURA, Zone::STACK);
    let sorcery = put(&mut g, &SORCERY, Zone::STACK);
    assert!(spell_can_target(&g, spell, bear), "an Aura spell's targets are its enchant ability's (CR 303.4a)");
    assert!(spell_can_target(&g, sorcery, bear));
    let copy = put(&mut g, &SOLE_AURA, Zone::STACK);
    assert!(spell_can_target(&g, copy, bear), "a second copy may target it too");
}

/// Casts the Aura card `card` from its owner's hand onto `host`, through the game's own priority and targeting: the targets the
/// spell was offered, and how many triggered abilities its cast put on the stack.
fn cast_onto(g: &mut Game, card: ObjectId, host: ObjectId) -> (Vec<Target>, usize) {
    g.run();
    let r = g.pending().cloned().expect("priority");
    let RequestKind::Priority { options } = &r.kind else { panic!("unexpected {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).expect("the Aura can be cast");
    let offered = option.target_candidates.to_vec();
    let mut action = Action::of(option);
    action.targets = vec![Target::Object(host)].into();
    g.answer(r.id, r.player, Answer::Act(action)).expect("cast");
    let r = g.pending().cloned().expect("priority");
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "unexpected {:?}", r.kind);
    assert!(g.state.stack.iter().any(|i| i.kind == StackKind::Spell), "the Aura spell is on the stack");
    (offered, g.state.stack.iter().filter(|i| i.kind == StackKind::Triggered).count())
}

/// Both players pass priority until the stack is empty.
fn resolve_all(g: &mut Game) {
    while !g.state.stack.is_empty() {
        let r = g.pending().cloned().expect("priority");
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "unexpected {:?}", r.kind);
        g.answer(r.id, r.player, Answer::Pass).expect("pass");
    }
}

/// Whether `spell` resolved (CR 608.2): the history's `Resolved` record of it.
fn resolved(g: &Game, spell: ObjectId) -> bool {
    g.log.iter().any(|r| matches!(r.event, Event::Resolved { source, .. } if source == spell))
}

/// Whether anything entered the battlefield from the stack as `spell` (CR 608.3).
fn entered_from(g: &Game, spell: ObjectId) -> bool {
    g.log.iter().any(|r| matches!(r.event, Event::ZoneChange { old, to, .. } if old == spell && to.is(ZoneKind::Battlefield)))
}

/// Another Aura spell targeting the enchanted creature is cast (an "enchantment spell cast" trigger goes on the stack) and
/// resolves, its target still legal; as it can't legally enchant it, it doesn't enter the battlefield - not even for a moment
/// before the state-based actions - and is put into its owner's graveyard instead (CR 303.4i, 608.3c). The granting Aura stays.
#[test]
fn another_aura_spell_resolving_onto_the_permanent_goes_to_the_graveyard() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let sole = attached(&mut g, &SOLE_AURA, bear);
    put(&mut g, &WATCHER, Zone::BATTLEFIELD);
    let card = put(&mut g, &PLAIN_AURA, Zone::hand(YOU));
    let (offered, triggered) = cast_onto(&mut g, card, bear);
    assert!(offered.contains(&Target::Object(bear)), "it may target the creature (CR 303.4a)");
    assert_eq!(triggered, 1, "it was cast: the watcher triggered");
    let spell = g.state.obj(card).became.expect("it is on the stack");
    resolve_all(&mut g);
    assert!(resolved(&g, spell), "its target is still legal: it resolves (CR 608.3b)");
    assert!(!entered_from(&g, spell), "it never enters the battlefield (CR 303.4i)");
    let gone = g.state.obj(spell).became.expect("it left the stack");
    assert!(g.state.obj(gone).zone.is(ZoneKind::Graveyard), "it is put into its owner's graveyard");
    assert!(stays(&g, sole, bear));
}

/// The same Aura spell onto a creature nothing restricts resolves and enters attached to it (CR 608.3c).
#[test]
fn another_aura_spell_resolving_onto_a_free_creature_enters_attached() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SOLE_AURA, bear);
    let card = put(&mut g, &PLAIN_AURA, Zone::hand(YOU));
    cast_onto(&mut g, card, free);
    let spell = g.state.obj(card).became.expect("it is on the stack");
    resolve_all(&mut g);
    let aura = g.state.obj(spell).became.expect("it left the stack");
    assert!(stays(&g, aura, free));
}

/// Resolves the spell `spell` on the stack with `targets`, as its own resolution frame would.
fn resolve_spell(g: &mut Game, def: &'static CardDef, spell: ObjectId, targets: &[Target]) {
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(g.cards.id_of(def)), YOU, targets);
    g.state.stack_push(item);
    let serial = g.state.stack.last().expect("the spell").serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    let frame = g.state.agenda.len() - 1;
    let mut cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    crate::resolve::resolve(&mut cx).expect("nothing is asked");
}

/// A copy of an Aura spell becomes a token as it resolves (CR 707.10f); onto the enchanted creature, its target still legal, the
/// token isn't created (CR 303.4i). Onto a free creature it is, attached.
#[test]
fn a_copy_of_another_aura_spell_resolving_onto_the_permanent_makes_no_token() {
    for restricted in [true, false] {
        let mut g = game();
        let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
        let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
        attached(&mut g, &SOLE_AURA, bear);
        let host = if restricted { bear } else { free };
        let copy = put(&mut g, &PLAIN_AURA, Zone::STACK);
        g.state.update_obj(copy, |o| o.card_copy = true);
        let before = g.state.battlefield().len();
        resolve_spell(&mut g, &PLAIN_AURA, copy, &[Target::Object(host)]);
        assert!(g.state.stack.is_empty());
        assert!(resolved(&g, copy), "its target is still legal: it resolves");
        if restricted {
            assert_eq!(g.state.battlefield().len(), before, "no token is created (CR 303.4i)");
        } else {
            let token = g.state.obj(copy).became.expect("it became a token");
            assert!(stays(&g, token, free));
        }
    }
}

/// A bestowed Aura spell (CR 702.103b), as a bestow cast leaves it on the stack, targeting `host`, resolved.
fn resolve_bestowed(g: &mut Game, host: ObjectId) -> ObjectId {
    let spell = put(g, &BESTOWER, Zone::STACK);
    let face = g.cards.id_of(&BESTOWED);
    g.state.update_obj(spell, |o| o.copy = Some(Copiable::of(face)));
    let mut item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(face), YOU, &[Target::Object(host)]);
    item.way = WAY_BESTOW;
    g.state.stack_push(item);
    let serial = g.state.stack.last().expect("the spell").serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    let frame = g.state.agenda.len() - 1;
    let mut cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    crate::resolve::resolve(&mut cx).expect("nothing is asked");
    spell
}

/// A bestowed Aura spell ceases to be bestowed only if its target is illegal (CR 702.103e, 608.3b). Its target the enchanted
/// creature, still legal, it resolves as the Aura spell it is (CR 608.3c), can't legally enchant it, and is put into its owner's
/// graveyard instead of entering (CR 303.4i) - it doesn't enter as a creature. Onto a free creature it enters attached; onto
/// one with protection from it, its target illegal, it resolves as a creature spell and enters unattached.
#[test]
fn a_bestowed_aura_spell_onto_the_permanent_goes_to_the_graveyard() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let sole = attached(&mut g, &SOLE_AURA, bear);
    let spell = resolve_bestowed(&mut g, bear);
    assert!(g.state.stack.is_empty());
    assert!(resolved(&g, spell), "its target is still legal: it resolves (CR 608.3b)");
    assert!(!entered_from(&g, spell), "it never enters the battlefield, as an Aura or a creature (CR 303.4i)");
    let gone = g.state.obj(spell).became.expect("it left the stack");
    assert!(g.state.obj(gone).zone.is(ZoneKind::Graveyard), "it is put into its owner's graveyard");
    assert!(stays(&g, sole, bear));

    let mut g = game();
    let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let spell = resolve_bestowed(&mut g, free);
    let aura = g.state.obj(spell).became.expect("it left the stack");
    assert!(stays(&g, aura, free), "onto a free creature it enters attached");

    let mut g = game();
    let warded = put(&mut g, &WARDED_BEAR, Zone::BATTLEFIELD);
    let spell = resolve_bestowed(&mut g, warded);
    let creature = g.state.obj(spell).became.expect("it left the stack");
    let o = g.state.obj(creature);
    assert!(o.zone.is(ZoneKind::Battlefield) && o.attached_to.is_none(), "its target illegal, it enters unattached (CR 702.103e)");
    assert!(cx(&mut g).chars(creature).is(CardType::Creature), "as the creature it is");
}

/// Protection is different: an Aura spell whose target has protection from it has an illegal target (CR 702.16b), so it
/// doesn't resolve at all (CR 608.3b) and is put into its owner's graveyard.
#[test]
fn an_aura_spell_onto_a_creature_with_protection_from_it_does_not_resolve() {
    let mut g = game();
    let bear = put(&mut g, &WARDED_BEAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &PLAIN_AURA, Zone::STACK);
    resolve_spell(&mut g, &PLAIN_AURA, spell, &[Target::Object(bear)]);
    assert!(g.state.stack.is_empty());
    assert!(!resolved(&g, spell), "its target is illegal: it doesn't resolve (CR 608.3b)");
    assert!(!entered_from(&g, spell));
    let gone = g.state.obj(spell).became.expect("it left the stack");
    assert!(g.state.obj(gone).zone.is(ZoneKind::Graveyard));
}

/// Another Aura can't enter attached to it, named (CR 303.4i: it stays where it is) or chosen (CR 303.4f).
#[test]
fn another_aura_can_not_enter_attached_to_the_permanent() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SOLE_AURA, bear);
    let card = put(&mut g, &PLAIN_AURA, Zone::graveyard(YOU));
    assert!(!cantrip_api::Game::could_enter_attached(&mut cx(&mut g), card, YOU, bear));
    assert!(cantrip_api::Game::could_enter_attached(&mut cx(&mut g), card, YOU, free));
    let entered = cantrip_api::Game::put_onto_battlefield_attached(&mut cx(&mut g), card, YOU, bear).expect("nothing is asked");
    assert_eq!(entered, None, "it can't enter attached (CR 303.4i)");
    assert!(g.state.obj(card).zone.is(ZoneKind::Graveyard), "it stays where it is");
    let cx = cx(&mut g);
    let chars = cx.chars(card);
    let hosts = crate::attach::aura_hosts(&cx, card, &chars, YOU, cx.mana_value(card), &[]);
    assert!(!hosts.contains(&Target::Object(bear)) && hosts.contains(&Target::Object(free)), "it can't be chosen (CR 303.4f)");
}

/// Another Aura on the battlefield can't be moved onto it (CR 303.4j: the Aura doesn't move); an Equipment, no Aura, can.
#[test]
fn another_aura_can_not_move_onto_the_permanent() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SOLE_AURA, bear);
    let other = attached(&mut g, &PLAIN_AURA, free);
    let equipment = put(&mut g, &EQUIPMENT, Zone::BATTLEFIELD);
    assert!(!can_attach(&mut g, other, bear));
    assert!(!cantrip_api::Game::attach(&mut cx(&mut g), other, bear), "it doesn't move");
    assert!(stays(&g, other, free));
    assert!(can_attach(&mut g, equipment, bear), "an Equipment is no Aura");
}

/// An Aura already attached is put into its owner's graveyard (CR 704.5m; Consecrate Land's ruling); the granting Aura stays.
#[test]
fn an_aura_already_attached_goes_and_the_granting_aura_stays() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let other = attached(&mut g, &PLAIN_AURA, bear);
    let sole = attached(&mut g, &SOLE_AURA, bear);
    assert!(crate::attach::host_allows(&cx(&mut g), sole, bear, false), "the granting Aura is no other Aura");
    state_based_actions(&mut g);
    assert!(stays(&g, sole, bear));
    assert!(went_to_the_graveyard(&g, other));
}

/// A second copy is "another Aura": it can't be attached, and two attached at once each make the other illegal - both go
/// (CR 704.5m, performed simultaneously, CR 704.3).
#[test]
fn a_second_copy_is_refused_and_two_attached_remove_each_other() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SOLE_AURA, bear);
    let second = attached(&mut g, &SOLE_AURA, free);
    assert!(!can_attach(&mut g, second, bear), "the second copy can't be attached");
    state_based_actions(&mut g);
    assert!(stays(&g, second, free), "each restricts only its own host");

    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let first = attached(&mut g, &SOLE_AURA, bear);
    let second = attached(&mut g, &SOLE_AURA, bear);
    state_based_actions(&mut g);
    assert!(went_to_the_graveyard(&g, first) && went_to_the_graveyard(&g, second), "both go");
}

/// The rule follows the granting Aura: moved to another creature, it restricts that one and frees the first; gone from the
/// battlefield, it restricts none.
#[test]
fn the_restriction_follows_the_aura_and_ends_as_it_leaves() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let free = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let sole = attached(&mut g, &SOLE_AURA, bear);
    let loose = put(&mut g, &PLAIN_AURA, Zone::BATTLEFIELD);
    assert!(!can_attach(&mut g, loose, bear));
    assert!(cantrip_api::Game::attach(&mut cx(&mut g), sole, free), "the granting Aura moves");
    assert!(can_attach(&mut g, loose, bear), "the first creature is free");
    assert!(!can_attach(&mut g, loose, free), "the second is not");
    // Unattached, it is put into its owner's graveyard (CR 704.5m), and so is the loose Aura.
    cantrip_api::Game::unattach(&mut cx(&mut g), sole);
    state_based_actions(&mut g);
    assert!(went_to_the_graveyard(&g, sole));
    let loose = put(&mut g, &PLAIN_AURA, Zone::BATTLEFIELD);
    assert!(can_attach(&mut g, loose, free), "it has left the battlefield");
    let card = put(&mut g, &PLAIN_AURA, Zone::graveyard(YOU));
    assert!(cantrip_api::Game::could_enter_attached(&mut cx(&mut g), card, YOU, free));
}

/// A rule of the Aura's, not an ability of the creature (CR 113.1a): the creature losing all its abilities is still under it;
/// the Aura losing all its abilities ends it (CR 613.1f).
#[test]
fn the_aura_losing_its_abilities_ends_it_and_the_creature_losing_its_abilities_does_not() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let sole = attached(&mut g, &SOLE_AURA, bear);
    let loose = put(&mut g, &PLAIN_AURA, Zone::BATTLEFIELD);
    let lose_all = |g: &mut Game, affected: ObjectId| {
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect {
            source: affected,
            controller: YOU,
            timestamp,
            affected,
            kind: EffectKind::LoseAllAbilities,
            duration: Duration::Indefinite,
            words: Words::NONE,
        });
    };
    lose_all(&mut g, bear);
    assert!(!can_attach(&mut g, loose, bear), "the creature's abilities are not the rule");
    lose_all(&mut g, sole);
    assert!(can_attach(&mut g, loose, bear), "the Aura has no static abilities (CR 613.1f)");
}
