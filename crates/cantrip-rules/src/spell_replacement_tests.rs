//! A granted way's "if you cast a spell this way, ..." (`GrantedWay::spell_replacement`, CR 614.1), over test cards: the
//! replacement effect is made about the spell as it is cast by the way, and applies to it whether or not the permanent
//! that gave the way is still there, or is still what it was (the rulings on Kess, Dissident Mage and Lier, Disciple of
//! the Drowned). A spell cast another way has none; a copy of the spell was not cast (CR 707.10) and has none; and it ends
//! with the spell (CR 400.7), so nothing else is touched by it. Leonardo, Sewer Samurai's "that creature enters with a
//! finality counter on it" and Kess, Dissident Mage's "if a spell cast this way would be put into your graveyard, exile it
//! instead" are the shapes. A top-of-library rule's rider (`PlayRule::LibraryTopSpells`'s second field) is the same effect:
//! Mikey & Don, Party Planners' "if you cast a creature spell this way, that creature enters with an additional +1/+1
//! counter on it".

use crate::{Cx, Game, Setup, ops, resolve};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Rider Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A 1/1 creature for {G}.
static SQUIRREL: CardDef = CardDef {
    name: "Rider Test Squirrel",
    cost: mana("{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// An instant for {U} that does nothing.
static NOTHING: CardDef = CardDef { name: "Rider Test Nothing", cost: mana("{U}"), types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };

/// A creature with nothing to give, for the giver to become a copy of.
static PLAIN: CardDef = CardDef {
    name: "Rider Test Plain",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn creature_card(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    let c = q.chars(card);
    (player == asker.controller && c.is(CardType::Creature)).then(|| Cost::mana(q.card(c.card).cost))
}

fn spell_enters(_: &dyn Query, _: &Asker, spell: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.object == spell && z.from.is(ZoneKind::Stack) && z.to.is(ZoneKind::Battlefield))
}

fn add_a_finality_counter(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::Finality as usize] += 1;
    }
}

/// "You may cast creature spells from your graveyard. If you cast a spell this way, that creature enters with a finality
/// counter on it" (Leonardo, Sewer Samurai's shape).
static LEONARDO: CardDef = CardDef {
    name: "Rider Test Leonardo",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    granted_ways: &[GrantedWay {
        text: "Cast a creature spell from your graveyard.",
        cost: creature_card,
        permission: true,
        spell_replacement: Some(0),
        ..GrantedWay::CAST
    }],
    granted_replacements: &[granted_replacement(ReplacementKind::ZoneChange, spell_enters, add_a_finality_counter, "enters with a finality counter")],
    ..CardDef::BLANK
};

fn instant_card(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    let c = q.chars(card);
    (player == asker.controller && c.is(CardType::Instant)).then(|| Cost::mana(q.card(c.card).cost))
}

fn spell_to_your_graveyard(_: &dyn Query, asker: &Asker, spell: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.object == spell && z.from.is(ZoneKind::Stack) && z.to == Zone::graveyard(asker.controller))
}

fn to_exile(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::exile(z.to.player);
    }
}

/// "You may cast an instant spell from your graveyard. If a spell cast this way would be put into your graveyard, exile it
/// instead" (Kess, Dissident Mage's shape).
static KESS: CardDef = CardDef {
    name: "Rider Test Kess",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(4),
    granted_ways: &[GrantedWay {
        text: "Cast an instant spell from your graveyard.",
        cost: instant_card,
        permission: true,
        spell_replacement: Some(0),
        ..GrantedWay::CAST
    }],
    granted_replacements: &[granted_replacement(ReplacementKind::ZoneChange, spell_to_your_graveyard, to_exile, "exiled instead")],
    ..CardDef::BLANK
};

fn creature_spell(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Creature)
}

/// A creature spell entering from the stack (Mikey & Don's "if you cast a creature spell this way, that creature enters").
fn creature_spell_enters(q: &dyn Query, _: &Asker, spell: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.object == spell && z.from.is(ZoneKind::Stack) && z.to.is(ZoneKind::Battlefield) && q.chars(spell).is(CardType::Creature))
}

fn add_a_plus_one(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += 1;
    }
}

/// "You may cast creature spells from the top of your library. If you cast a creature spell this way, that creature enters
/// with an additional +1/+1 counter on it" (Mikey & Don, Party Planners' shape).
static MIKEY: CardDef = CardDef {
    name: "Rider Test Mikey",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    play_rules: &[PlayRule::LibraryTopSpells(creature_spell, Some(0))],
    granted_replacements: &[granted_replacement(ReplacementKind::ZoneChange, creature_spell_enters, add_a_plus_one, "enters with a +1/+1 counter")],
    ..CardDef::BLANK
};

/// "You may cast creature spells from the top of your library", and no rider (Garruk's Horde's shape).
static HORDE: CardDef = CardDef {
    name: "Rider Test Horde",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(7),
    toughness: Some(7),
    play_rules: &[PlayRule::LibraryTopSpells(creature_spell, None)],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Your main phase with priority and an empty stack.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &SQUIRREL, &NOTHING, &PLAIN, &LEONARDO, &KESS, &MIKEY, &HORDE])));
    let deck = || vec![table.id_of(&FILLER); 20];
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
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

/// Puts a card of `def` on top of your library.
fn put_on_top(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let zone = Zone::library(YOU);
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_insert(zone, 0, id);
    id
}

fn add(g: &mut Game, mana: Mana, n: u32) {
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, mana, n, source, None, ManaLasts::Step, false);
    ask_again(g);
}

/// Does `what` to the game between questions, as an effect would, and has the priority question asked again of the game it
/// leaves: an answer to the question asked before would redo its frame from where it was asked, undoing `what`.
fn between<T>(g: &mut Game, what: impl FnOnce(&mut Cx) -> T) -> T {
    let out = what(&mut cx(g));
    ask_again(g);
    out
}

fn ask_again(g: &mut Game) {
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// Casts `card` - by the way a permanent gives, or as it may be cast without one - and pays from the pool.
fn cast(g: &mut Game, card: ObjectId, granted: bool) -> ObjectId {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.granted_by.is_some() == granted && o.paid_by.is_none())
        .expect("the cast is offered")
        .clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    g.state.stack.last().expect("the spell is on the stack").object
}

/// Casts `card` from the top of the library by the rule of the permanent `by`, and pays from the pool.
fn cast_by(g: &mut Game, card: ObjectId, by: ObjectId) -> ObjectId {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.granted_by.is_some_and(|g| g.object == by))
        .expect("the cast is offered")
        .clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    g.state.stack.last().expect("the spell is on the stack").object
}

fn resolve_all(g: &mut Game) {
    while !g.state.stack.is_empty() {
        let r = next(g);
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "unexpected {:?}", r.kind);
        g.answer(r.id, r.player, Answer::Pass).expect("a legal answer");
    }
    g.run();
}

fn finality(g: &Game, permanent: ObjectId) -> i32 {
    g.state.obj(permanent).counters(CounterKind::Finality)
}

fn riders(g: &Game) -> usize {
    g.state.effects.iter().filter(|e| matches!(e.kind, EffectKind::Replacement { .. })).count()
}

/// "That creature enters with a finality counter on it" (CR 614.1c, 122.6): the creature cast by the way enters with one.
#[test]
fn a_creature_cast_by_the_way_enters_with_the_counter() {
    let mut g = game();
    put(&mut g, &LEONARDO, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::graveyard(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, true);
    assert_eq!(riders(&g), 1, "the effect is made as the spell is cast");
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert_eq!(finality(&g, permanent), 1);
    assert_eq!(riders(&g), 0, "the effect ended with the spell");
}

/// The owner's ruling (as Kess's and Lier's): the rider follows the spell, not the permission's source. The permanent that
/// gave the way has left the battlefield before the spell resolves, and the creature still enters with the counter.
#[test]
fn the_rider_applies_after_the_giver_has_left() {
    let mut g = game();
    let leonardo = put(&mut g, &LEONARDO, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::graveyard(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, true);
    between(&mut g, |cx| ops::move_to(cx, leonardo, Zone::graveyard(YOU)));
    resolve_all(&mut g);
    assert!(g.state.obj(leonardo).became.is_some_and(|gone| g.state.obj(gone).zone.is(ZoneKind::Graveyard)), "the giver left first");
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(finality(&g, permanent), 1);
}

/// The effect names the card whose way it is: the giver becoming a copy of a card with no such effect after the cast
/// (CR 707.2) changes nothing, and reads nothing out of that card.
#[test]
fn the_rider_is_the_ways_card_whatever_the_giver_becomes() {
    let mut g = game();
    let leonardo = put(&mut g, &LEONARDO, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::graveyard(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, true);
    let plain = g.cards.id_of(&PLAIN);
    g.state.update_obj(leonardo, |o| o.copy = Some(Copiable::of(plain)));
    ask_again(&mut g);
    resolve_all(&mut g);
    assert_eq!(g.state.obj(leonardo).copied_card(), Some(plain), "the giver became a copy first");
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(finality(&g, permanent), 1);
}

/// A spell cast another way - from its owner's hand, with the giver on the battlefield - was not cast this way.
#[test]
fn a_creature_cast_another_way_enters_without_it() {
    let mut g = game();
    put(&mut g, &LEONARDO, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::hand(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, false);
    assert_eq!(riders(&g), 0);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(finality(&g, permanent), 0);
}

/// A copy of the spell was not cast (CR 707.10), so it was not cast this way: the token it becomes has no counter, and
/// the original still enters with one.
#[test]
fn a_copy_of_the_spell_has_no_rider() {
    let mut g = game();
    put(&mut g, &LEONARDO, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::graveyard(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, true);
    let copy = between(&mut g, |cx| ops::copy_spell(cx, spell, YOU, false)).ok().flatten().expect("the spell is copied");
    resolve_all(&mut g);
    let token = *g.state.battlefield().iter().find(|&&id| g.state.obj(id).token).expect("the copy resolved into a token");
    assert_ne!(token, copy);
    assert_eq!(finality(&g, token), 0, "the copy was not cast this way");
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(finality(&g, permanent), 1);
}

/// A spell countered goes to its graveyard as a new object (CR 400.7) the effect knows nothing of: it ends there, and the
/// card cast again from the graveyard another way - from the hand it is returned to - enters with no counter.
#[test]
fn a_countered_spell_takes_its_rider_with_it() {
    let mut g = game();
    put(&mut g, &LEONARDO, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::graveyard(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, true);
    between(&mut g, |cx| resolve::counter(cx, spell));
    let countered = g.state.obj(spell).became.expect("it was countered");
    assert!(g.state.obj(countered).zone.is(ZoneKind::Graveyard), "a creature spell's rider does not touch its being countered");
    let back = between(&mut g, |cx| ops::move_to(cx, countered, Zone::hand(YOU)));
    add(&mut g, Mana::G, 1);
    let again = cast(&mut g, back, false);
    resolve_all(&mut g);
    let permanent = g.state.obj(again).became.expect("it resolved");
    assert_eq!(finality(&g, permanent), 0, "the second cast was not this way");
    assert_eq!(riders(&g), 0, "the effect ended with the spell it was about");
}

/// Kess's shape: an instant cast this way that would be put into its owner's graveyard as it resolves is exiled instead,
/// though the permanent that gave the way has left.
#[test]
fn an_instant_cast_this_way_is_exiled_as_it_resolves() {
    let mut g = game();
    let kess = put(&mut g, &KESS, Zone::BATTLEFIELD);
    let card = put(&mut g, &NOTHING, Zone::graveyard(YOU));
    add(&mut g, Mana::U, 1);
    let spell = cast(&mut g, card, true);
    between(&mut g, |cx| ops::move_to(cx, kess, Zone::graveyard(YOU)));
    resolve_all(&mut g);
    assert!(g.state.obj(kess).became.is_some(), "the giver left first");
    let after = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(after).zone.is(ZoneKind::Exile));
}

/// Kess's shape, countered: a spell cast this way that would be put into its graveyard by being countered is exiled too
/// (CR 614.1a: the replacement watches the spell, whatever puts it there).
#[test]
fn an_instant_cast_this_way_and_countered_is_exiled() {
    let mut g = game();
    put(&mut g, &KESS, Zone::BATTLEFIELD);
    let card = put(&mut g, &NOTHING, Zone::graveyard(YOU));
    add(&mut g, Mana::U, 1);
    let spell = cast(&mut g, card, true);
    between(&mut g, |cx| resolve::counter(cx, spell));
    let after = g.state.obj(spell).became.expect("it was countered");
    assert!(g.state.obj(after).zone.is(ZoneKind::Exile));
}

/// Kess's shape, cast from the hand: not this way, so it goes to the graveyard as it resolves.
#[test]
fn an_instant_cast_another_way_goes_to_the_graveyard() {
    let mut g = game();
    put(&mut g, &KESS, Zone::BATTLEFIELD);
    let card = put(&mut g, &NOTHING, Zone::hand(YOU));
    add(&mut g, Mana::U, 1);
    let spell = cast(&mut g, card, false);
    resolve_all(&mut g);
    let after = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(after).zone.is(ZoneKind::Graveyard));
}

fn plus_ones(g: &Game, permanent: ObjectId) -> i32 {
    g.state.obj(permanent).counters(CounterKind::PlusOne)
}

/// Mikey & Don's shape: a creature cast from the top of the library by the rule enters with the counter, the effect made as
/// it is cast and ended with it.
#[test]
fn a_creature_cast_from_the_top_by_the_rule_enters_with_the_counter() {
    let mut g = game();
    put(&mut g, &MIKEY, Zone::BATTLEFIELD);
    add(&mut g, Mana::G, 1);
    let card = put_on_top(&mut g, &SQUIRREL);
    ask_again(&mut g);
    let spell = cast(&mut g, card, true);
    assert_eq!(riders(&g), 1, "the effect is made as the spell is cast");
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert_eq!(plus_ones(&g, permanent), 1);
    assert_eq!(riders(&g), 0, "the effect ended with the spell");
}

/// The owner's ruling, for the top-of-library rule too: the rule's permanent has left the battlefield before the spell
/// resolves, and the creature still enters with the counter.
#[test]
fn the_top_of_library_rider_applies_after_the_permanent_has_left() {
    let mut g = game();
    let mikey = put(&mut g, &MIKEY, Zone::BATTLEFIELD);
    add(&mut g, Mana::G, 1);
    let card = put_on_top(&mut g, &SQUIRREL);
    ask_again(&mut g);
    let spell = cast(&mut g, card, true);
    between(&mut g, |cx| ops::move_to(cx, mikey, Zone::hand(YOU)));
    resolve_all(&mut g);
    assert!(g.state.obj(mikey).became.is_some_and(|gone| g.state.obj(gone).zone.is(ZoneKind::Hand)), "the permanent left first");
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(plus_ones(&g, permanent), 1);
}

/// The same card cast from the top by another permanent's rule, one with no rider, was not cast "this way": no effect, no
/// counter, though the rider's permanent is on the battlefield.
#[test]
fn a_creature_cast_from_the_top_by_another_rule_enters_without_it() {
    let mut g = game();
    put(&mut g, &MIKEY, Zone::BATTLEFIELD);
    let horde = put(&mut g, &HORDE, Zone::BATTLEFIELD);
    add(&mut g, Mana::G, 1);
    let card = put_on_top(&mut g, &SQUIRREL);
    ask_again(&mut g);
    let spell = cast_by(&mut g, card, horde);
    assert_eq!(riders(&g), 0);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(plus_ones(&g, permanent), 0);
}

/// A creature cast from the hand, with the rule's permanent on the battlefield, was not cast this way.
#[test]
fn a_creature_cast_from_the_hand_beside_the_rule_enters_without_it() {
    let mut g = game();
    put(&mut g, &MIKEY, Zone::BATTLEFIELD);
    let card = put(&mut g, &SQUIRREL, Zone::hand(YOU));
    add(&mut g, Mana::G, 1);
    let spell = cast(&mut g, card, false);
    assert_eq!(riders(&g), 0);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_eq!(plus_ones(&g, permanent), 0);
}
