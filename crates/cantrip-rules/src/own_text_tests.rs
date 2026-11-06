//! A resolving ability reads its own text as it was put on the stack (CR 113.7a, `Cx::own_text`), and nothing else does: what
//! the engine asks of the source's other abilities in the middle of the resolution - what triggers, what replaces or prevents
//! an event, a state trigger, a triggered mana ability - is each one's own text. Printed abilities read the source's now;
//! resolved effects retain their creation text. Over a test card whose
//! text says "Goblin" and has been changed to say "Elf" since its ability went on the stack: each hook notes the word it
//! read; printed abilities read "Elf" while the resolved shield retains "Goblin". No printed card has an ability that makes its own source's other word-reading ability
//! look in the middle of its resolution, so these are the guards' only tests.

use crate::actions::stack_item;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
use std::cell::RefCell;

const YOU: PlayerId = PlayerId(0);
thread_local! {
    /// What each hook read "Goblin" as, by hook.
    static SEEN: RefCell<Vec<(&'static str, Subtype)>> = const { RefCell::new(Vec::new()) };
}

/// Notes the word `hook` reads its source's "Goblin" as.
fn note(hook: &'static str, q: &dyn Query, source: ObjectId) {
    let word = q.words(source).subtype(Subtype::Goblin);
    SEEN.with(|s| s.borrow_mut().push((hook, word)));
}

/// What `hook` last read "Goblin" as, if it was asked.
fn seen(hook: &'static str) -> Option<Subtype> {
    SEEN.with(|s| s.borrow().iter().rev().find(|(h, _)| *h == hook).map(|&(_, w)| w))
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// Notes the word as it is asked whether it triggers, and does not.
fn a_spell_cast(q: &dyn Query, source: ObjectId, _: &Event) -> Option<Fired> {
    note("trigger", q, source);
    None
}

fn always_on_a_spell(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    matches!(event, Event::SpellCast { .. }).then_some(Fired { subject: Target::None, data: [0; 2] })
}

fn members(q: &dyn Query, _: Target) -> Vec<ObjectId> {
    let source = q.state().battlefield()[0];
    note("members", q, source);
    Vec::new()
}

fn state(q: &dyn Query, asker: &Asker) -> bool {
    note("state", q, asker.source);
    false
}

fn tapped(q: &dyn Query, source: ObjectId, _: &Event) -> Option<Fired> {
    note("mana", q, source);
    None
}

fn always(_: &dyn Query, _: &Asker, _: &Proposal) -> bool {
    true
}

fn never(q: &dyn Query, asker: &Asker, _: &Proposal) -> bool {
    note("applies", q, asker.source);
    false
}

fn life_apply(q: &dyn Query, asker: &Asker, _: &mut Proposal) {
    note("life", q, asker.source);
}

fn token_apply(q: &dyn Query, asker: &Asker, _: &mut Proposal) {
    note("tokens", q, asker.source);
}

fn mana_apply(q: &dyn Query, asker: &Asker, _: &mut Proposal) {
    note("reshape", q, asker.source);
}

fn mana_only(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ManaProduction(_))
}

fn shielded(q: &dyn Query, asker: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    note("shield", q, asker.source);
    true
}

/// A Goblin whose every hook reads "Goblin" through its words.
static SOURCE: CardDef = CardDef {
    name: "Own Text Goblin",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        triggered(when(EventMask::TAPPED, tapped), nothing, "Whenever this is tapped for mana, if Goblin, add {G}.").mana_ability(TriggeredMana::One(Mana::G)),
        triggered(when(EventMask::SPELL_CAST, a_spell_cast), nothing, "Whenever a Goblin spell is cast, do nothing."),
        triggered(when(EventMask::SPELL_CAST, always_on_a_spell), nothing, "Whenever a spell is cast, do nothing for each Goblin.").members_of(members),
        state_triggered(state, nothing, "When you control a Goblin, do nothing."),
    ],
    replacements: &[
        replacement(ReplacementKind::LifeChange, always, life_apply, "Goblin life"),
        replacement(ReplacementKind::TokenCreation, always, token_apply, "Goblin tokens"),
        replacement(ReplacementKind::ManaProduction, never, mana_apply, "Goblin mana, never"),
        replacement(ReplacementKind::ManaProduction, mana_only, mana_apply, "Goblin mana"),
    ],
    granted_preventions: &[GrantedPreventionDef { hint: "Goblin shield", damage: "damage", of_it: None, applies: shielded, then: None, text: "" }],
    delayed: &[triggered(when(EventMask::SPELL_CAST, delayed_cast), nothing, "This turn, whenever a Goblin spell is cast, do nothing.")],
    ..CardDef::BLANK
};

/// The delayed ability's trigger event: notes the word, and triggers on any spell.
fn delayed_cast(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    note("delayed", q, source);
    matches!(event, Event::SpellCast { .. }).then_some(Fired { subject: Target::None, data: [0; 2] })
}

static TOKEN: CardDef = CardDef { name: "Own Text Token", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

/// The Goblin on the battlefield, its text changed to say Elf, and its ability on the stack resolving with the text it was
/// put there with, printed: the frame `Cx::own_text` is read in.
fn resolving() -> (Game, ObjectId, usize) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&SOURCE, &TOKEN])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let card = table.id_of(&SOURCE);
    let id = g.state.push_obj(Obj::new(card, YOU, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    let item = stack_item(StackKind::Triggered, id, AbilityRef { card, index: 2 }, YOU, &[]);
    g.state.stack_push(item);
    let serial = g.state.stack.last().expect("the ability").serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    let frame = g.state.agenda.len() - 1;
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect {
        source: id,
        controller: YOU,
        timestamp,
        affected: id,
        kind: EffectKind::ChangeText { from: Word::Subtype(Subtype::Goblin), to: Word::Subtype(Subtype::Elf) },
        duration: Duration::Indefinite,
        words: Words::NONE,
    });
    SEEN.with(|s| s.borrow_mut().clear());
    (g, id, frame)
}

/// The resolving ability's own text is read: its source says Goblin to it, though it says Elf now.
fn own(g: &mut Game, frame: usize) -> Cx<'_> {
    g.own_text.set(true);
    Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn reads_its_own_text(cx: &Cx, id: ObjectId) {
    assert_eq!(Query::words(cx, id).subtype(Subtype::Goblin), Subtype::Goblin, "the resolving ability reads the text it was put on the stack with");
}

#[test]
fn what_triggers_is_asked_of_the_text_now() {
    let (mut g, id, frame) = resolving();
    let mut cx = own(&mut g, frame);
    reads_its_own_text(&cx, id);
    cx.emit(Event::SpellCast { spell: id, player: YOU, nth: 1, options: 0 });
    assert_eq!(seen("trigger"), Some(Subtype::Elf));
    reads_its_own_text(&cx, id);
}

/// The objects a triggered ability is about (`TriggeredDef::members_of`) are found as it triggers, after the trigger events
/// are looked at, by its own text now - the guard around `members_of` in `settle::collect_batch`.
#[test]
fn the_objects_a_trigger_is_about_are_found_by_the_text_now() {
    let (mut g, id, frame) = resolving();
    let mut cx = own(&mut g, frame);
    cx.emit(Event::SpellCast { spell: id, player: YOU, nth: 1, options: 0 });
    assert_eq!(seen("members"), Some(Subtype::Elf));
    reads_its_own_text(&cx, id);
}

#[test]
fn a_state_trigger_is_asked_of_the_text_now() {
    let (mut g, id, frame) = resolving();
    let mut cx = own(&mut g, frame);
    crate::settle::state_triggers_now(&mut cx);
    assert_eq!(seen("state"), Some(Subtype::Elf));
    reads_its_own_text(&cx, id);
}

#[test]
fn a_replacement_applies_by_the_text_now() {
    let (mut g, id, frame) = resolving();
    let mut cx = own(&mut g, frame);
    crate::replace::life_change(&mut cx, YOU, 1);
    assert_eq!(seen("life"), Some(Subtype::Elf));
    reads_its_own_text(&cx, id);
}

#[test]
fn a_replacement_of_a_token_batch_applies_by_the_text_now() {
    let (mut g, id, frame) = resolving();
    let token = g.cards.id_of(&TOKEN);
    let mut cx = own(&mut g, frame);
    crate::replace::token_batch(&mut cx, &[(token, None, YOU)]);
    assert_eq!(seen("tokens"), Some(Subtype::Elf));
    reads_its_own_text(&cx, id);
}

#[test]
fn the_outcomes_of_a_production_are_judged_by_the_text_now() {
    let (mut g, id, frame) = resolving();
    let cx = own(&mut g, frame);
    let production = ManaProduction {
        permanent: id,
        player: YOU,
        to: YOU,
        mana: ManaPool::default(),
        restriction: None,
        lasts: ManaLasts::default(),
    };
    crate::replace::mana_outcomes(&cx, production);
    assert_eq!(seen("applies"), Some(Subtype::Elf), "whether an effect applies");
    assert_eq!(seen("reshape"), Some(Subtype::Elf), "what an effect does to the outcome");
    reads_its_own_text(&cx, id);
}

#[test]
fn a_resolved_prevention_keeps_its_creation_text_while_other_abilities_read_theirs() {
    let (mut g, id, frame) = resolving();
    let card = g.cards.id_of(&SOURCE);
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect {
        source: id,
        controller: YOU,
        timestamp,
        affected: id,
        kind: EffectKind::Prevention { card, index: 0, to: Target::None, shield: Shield::Next(3) },
        duration: Duration::EndOfTurn,
        words: Words::NONE,
    });
    let cx = own(&mut g, frame);
    let damage = DamageEvent { source: id, target: Target::Player(YOU), amount: 2, combat: false, heal_marked: false, shielded: None };
    let shields = crate::replace::shields(&cx, &damage);
    assert_eq!(seen("shield"), Some(Subtype::Goblin), "the resolved effect has its own unchanged text: {shields:?}");
    reads_its_own_text(&cx, id);
}

/// A delayed triggered ability the resolving ability creates is that ability's text (CR 603.7, 113.7a): it keeps the words
/// the ability was put on the stack with, though its source says Elf, and reads them when it is asked whether it triggers
/// and once it is on the stack - after the resolution, with nothing of the ability's own text read any more.
#[test]
fn a_delayed_ability_keeps_the_text_that_created_it() {
    let (mut g, id, frame) = resolving();
    let card = g.cards.id_of(&SOURCE);
    let mut cx = own(&mut g, frame);
    cantrip_api::Game::delayed_trigger(&mut cx, DelayedWhen::ThisTurn, id, YOU, AbilityRef::delayed(card, 0), Target::None);
    cx.g.own_text.set(false);
    assert_eq!(Query::words(&cx, id).subtype(Subtype::Goblin), Subtype::Elf, "the source says Elf to anything else");
    cx.emit(Event::SpellCast { spell: id, player: YOU, nth: 1, options: 0 });
    assert_eq!(seen("delayed"), Some(Subtype::Goblin), "asked whether it triggers by the text that created it");
    let pending = cx.s().triggers.iter().find(|t| t.ability == AbilityRef::delayed(card, 0)).expect("it triggered");
    assert_eq!(pending.words.map(|w| w.subtype(Subtype::Goblin)), Some(Subtype::Goblin), "and put on the stack with it");
}

#[test]
fn a_mana_ability_activated_mid_resolution_is_its_own_text_now() {
    let (mut g, id, frame) = resolving();
    let mut cx = own(&mut g, frame);
    crate::actions::activate_mana(&mut cx, YOU, id, 0, None, false, &[]).expect("activated");
    assert_eq!(seen("mana"), Some(Subtype::Elf), "the triggered mana ability it triggers");
    reads_its_own_text(&cx, id);
}
