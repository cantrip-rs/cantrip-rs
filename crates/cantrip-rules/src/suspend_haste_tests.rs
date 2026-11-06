//! Suspend's haste (CR 702.62a), over a test card: a creature spell cast by suspend's last-counter trigger gains haste
//! until its caster loses control of the spell or the permanent it becomes. The spell has it on the stack, the permanent
//! has it once it resolves (CR 400.7a), and it is gone for good once another player gains control of the permanent, though
//! control come back (CR 611.2b).

use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Suspend Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A creature with suspend 1.
static SPRINTER: CardDef = CardDef {
    name: "Suspend Test Sprinter",
    cost: generic(3),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    suspend: Some(SuspendDef { count: 1, cost: ManaCost::ZERO }),
    ..CardDef::BLANK
};

/// An artifact with suspend 1: no creature spell, so nothing gains haste.
static TRINKET: CardDef = CardDef {
    name: "Suspend Test Trinket",
    cost: generic(3),
    types: TypeSet::of(&[CardType::Artifact]),
    suspend: Some(SuspendDef { count: 1, cost: ManaCost::ZERO }),
    ..CardDef::BLANK
};

fn spell_on_the_stack(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id))
}

/// "Gain control of target spell" (Aethersnatch's kind, CR 112.2).
fn steal(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(spell) = g.state().target(item, 0).object()
        && g.target_legal(item, 0)
    {
        g.change_controller(spell, item.controller);
    }
    Ok(())
}

static STEAL: CardDef = CardDef {
    name: "Suspend Test Steal",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(spell_on_the_stack)], steal),
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Your turn about to reach its upkeep, `def` suspended in your exile with its last time counter.
fn game(def: &'static CardDef) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &SPRINTER, &TRINKET, &STEAL])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::Untap);
    });
    let card = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::exile(YOU)));
    g.state.zone_push(Zone::exile(YOU), card);
    crate::counters::put(&mut cx(&mut g), card, CounterKind::Time, 1, YOU, crate::counters::How::Effect);
    (g, card)
}

/// Plays on, passing priority and casting the suspended card when asked, until `done` says so.
fn play_until(g: &mut Game, done: impl Fn(&Game) -> bool) {
    play_until_with(g, done, |_, _| None);
}

/// `play_until`, with `act` answering any priority it wants to.
fn play_until_with(g: &mut Game, done: impl Fn(&Game) -> bool, act: impl Fn(&Game, &Request) -> Option<Answer>) {
    for _ in 0..200 {
        g.run();
        if done(g) {
            return;
        }
        let r = g.pending().cloned().expect("a question");
        let answer = match &r.kind {
            RequestKind::Priority { .. } => act(g, &r).unwrap_or(Answer::Pass),
            RequestKind::ChooseTarget { candidates, .. } => Answer::Targets(vec![candidates[0]]),
            RequestKind::May { purpose: Purpose::CastFree, .. } => Answer::Yes,
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(r.id, r.player, answer).expect("a legal answer");
    }
    panic!("never got there");
}

fn has_haste(g: &mut Game, id: ObjectId) -> bool {
    cx(g).chars(id).has(Keyword::Haste)
}

fn spell_on_stack(g: &Game) -> Option<ObjectId> {
    g.state.stack.iter().find(|i| i.kind == StackKind::Spell).map(|i| i.object)
}

fn on_battlefield(g: &Game, def: &'static CardDef) -> Option<ObjectId> {
    g.state.battlefield().iter().copied().find(|&id| g.state.obj(id).card == g.cards.id_of(def))
}

#[test]
fn a_creature_cast_with_suspend_has_haste_on_the_stack_and_on_the_battlefield() {
    let (mut g, _) = game(&SPRINTER);
    play_until(&mut g, |g| spell_on_stack(g).is_some());
    let spell = spell_on_stack(&g).expect("cast");
    assert!(has_haste(&mut g, spell), "the spell gained haste");
    play_until(&mut g, |g| on_battlefield(g, &SPRINTER).is_some());
    let sprinter = on_battlefield(&g, &SPRINTER).expect("resolved");
    assert!(has_haste(&mut g, sprinter), "the permanent it became has haste");
    // It may attack the turn it came under its controller's control (CR 302.6, 702.10b).
    assert!(!crate::layers::summoning_sick(g.cards, &g.state, &g.log, sprinter));
}

#[test]
fn suspends_haste_ends_for_good_when_its_caster_loses_control() {
    let (mut g, _) = game(&SPRINTER);
    play_until(&mut g, |g| on_battlefield(g, &SPRINTER).is_some());
    let sprinter = on_battlefield(&g, &SPRINTER).expect("resolved");
    let mut c = cx(&mut g);
    cantrip_api::Game::add_effect(&mut c, sprinter, sprinter, EffectKind::Control(THEM), Duration::EndOfTurn);
    assert_eq!(c.s().obj(sprinter).controller, THEM);
    assert!(!c.chars(sprinter).has(Keyword::Haste), "its caster lost control: no haste");
    // Control comes back; the haste does not (CR 611.2b).
    let keep: Vec<bool> = c.s().effects.iter().map(|e| !matches!(e.kind, EffectKind::Control(_))).collect();
    c.st().effects_keep(&keep);
    crate::ops::settle_lasting_and_control(&mut c);
    assert_eq!(c.s().obj(sprinter).controller, YOU);
    assert!(!c.chars(sprinter).has(Keyword::Haste), "the haste is gone for good");
}

#[test]
fn a_noncreature_cast_with_suspend_gains_nothing() {
    let (mut g, _) = game(&TRINKET);
    play_until(&mut g, |g| on_battlefield(g, &TRINKET).is_some());
    assert!(g.state.effects.iter().all(|e| !matches!(e.kind, EffectKind::AddKeywords(_))));
}

/// Their Steal takes the Sprinter spell on the stack (CR 112.2): its caster no longer controls it, so its haste is over
/// (`caster_controls` reads the spell's controller now), and the creature it becomes under them has none.
#[test]
fn suspends_haste_ends_when_the_spell_changes_control_on_the_stack() {
    let (mut g, _) = game(&SPRINTER);
    let steal = g.state.push_obj(Obj::new(g.cards.id_of(&STEAL), THEM, Zone::hand(THEM)));
    g.state.zone_push(Zone::hand(THEM), steal);
    play_until(&mut g, |g| spell_on_stack(g).is_some());
    let spell = spell_on_stack(&g).expect("cast");
    assert!(has_haste(&mut g, spell));
    play_until_with(
        &mut g,
        |g| g.state.obj(spell).controller == THEM,
        |g, r| match &r.kind {
            RequestKind::Priority { options } if r.player == THEM && g.state.obj(steal).zone == Zone::hand(THEM) => {
                options.iter().find(|o| o.source == Some(steal)).map(|o| Answer::Act(Action { targets: o.target_candidates.clone(), ..Action::of(o) }))
            }
            _ => None,
        },
    );
    assert!(!has_haste(&mut g, spell), "its caster lost control of the spell: no haste");
    play_until(&mut g, |g| on_battlefield(g, &SPRINTER).is_some());
    let sprinter = on_battlefield(&g, &SPRINTER).expect("resolved");
    assert_eq!(g.state.obj(sprinter).controller, THEM);
    assert!(!has_haste(&mut g, sprinter));
}
