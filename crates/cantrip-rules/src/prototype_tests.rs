//! Prototype as the core does it (CR 702.160, 718), over test cards: a prototype card may be cast prototyped, for its
//! prototype's mana cost, as a spell with only the prototype's mana cost, power and toughness and the color that mana cost
//! gives (CR 718.3a-b), which resolves into a permanent with the same (CR 718.3b); everything else is the card's (CR 718.5).
//! In every other zone, and once it leaves the stack or the battlefield, it has its normal characteristics (CR 718.4). A
//! copy of a prototyped spell or permanent is prototyped too (CR 718.2a, 718.3c-d). Casting it prototyped is no alternative
//! cost, so an alternative cost may be applied to it (the ruling).

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Prototype Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Prototype {2}{R} — 3/2 / Haste": a 6/4 Construct for {7}.
static AUTOMATON: CardDef = CardDef {
    name: "Prototype Test Automaton",
    cost: generic(7),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Construct]),
    keywords: Keywords::of(&[Keyword::Haste]),
    power: Some(6),
    toughness: Some(4),
    text: "Prototype {2}{R} \u{2014} 3/2\nHaste",
    prototype: Some(Prototype { cost: mana("{2}{R}"), power: 3, toughness: 2, text: "Prototype {2}{R} \u{2014} 3/2" }),
    ..CardDef::BLANK
};

/// "You may cast spells from your hand without paying their mana costs" (Omniscience's, CR 118.9).
fn any_spell_you_cast(_: &dyn Query, asker: &Asker, _: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller
}
static OMNISCIENT: CardDef = CardDef {
    name: "Prototype Test Omniscient",
    types: TypeSet::of(&[CardType::Enchantment]),
    spell_alternatives: &[SpellAlternative { text: "Without paying its mana cost", applies: any_spell_you_cast, cost: InsteadOfManaCost::WithoutPaying, flash: false }],
    ..CardDef::BLANK
};

/// "You may cast permanent spells with mana value 3 or less from your graveyard": a permission (CR 601.3), the spell judged
/// as the face being cast (CR 718.3a).
fn cheap_permanent(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    let c = q.chars(card);
    (player == asker.controller && !c.is(CardType::Instant) && !c.is(CardType::Sorcery) && q.mana_value(card) <= 3).then(|| Cost::mana(q.card(c.card).cost))
}
static RECURSION: CardDef = CardDef {
    name: "Prototype Test Recursion",
    types: TypeSet::of(&[CardType::Enchantment]),
    granted_ways: &[GrantedWay { text: "Cast a cheap permanent from your graveyard.", cost: cheap_permanent, permission: true, ..GrantedWay::CAST }],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// Your main phase with priority and an empty stack.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &AUTOMATON, &OMNISCIENT, &RECURSION])));
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

fn add(g: &mut Game, mana: Mana, n: u32) {
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, mana, n, source, None, ManaLasts::Step);
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

fn options(g: &mut Game) -> Vec<ActionOption> {
    match next(g).kind {
        RequestKind::Priority { options } => options,
        other => panic!("expected priority, got {other:?}"),
    }
}

fn cast_option(g: &mut Game, card: ObjectId, way: u8, alternative: bool) -> Option<ActionOption> {
    options(g).into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == way && o.paid_by.is_some() == alternative)
}

/// Takes `option` and pays for it from the pool, giving back the next question.
fn take(g: &mut Game, option: &ActionOption) -> Request {
    let r = next(g);
    g.answer(r.id, r.player, Answer::Act(Action::of(option))).expect("a legal answer");
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { return r };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
}

fn cast(g: &mut Game, card: ObjectId, way: u8) -> ObjectId {
    let option = cast_option(g, card, way, false).expect("the cast is offered");
    take(g, &option);
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

/// The card's normal characteristics (CR 718.4): colorless, mana value 7, 6/4.
fn assert_normal(g: &Game, id: ObjectId) {
    let c = g.chars(id);
    assert_eq!((c.power, c.toughness), (Some(6), Some(4)));
    assert!(c.colors.is_empty());
    assert_eq!(Query::mana_value(g, id), 7);
    assert_eq!(Query::mana_cost(g, id), generic(7));
}

/// The prototype's (CR 718.3b): red, mana value 3, 3/2 - and everything else the card's (CR 718.5).
fn assert_prototyped(g: &Game, id: ObjectId) {
    let c = g.chars(id);
    assert_eq!((c.power, c.toughness), (Some(3), Some(2)));
    assert_eq!(c.colors, ColorSet::of(Color::Red));
    assert_eq!(Query::mana_value(g, id), 3);
    assert_eq!(Query::mana_cost(g, id), mana("{2}{R}"));
    assert_eq!(c.name, "Prototype Test Automaton");
    assert_eq!(c.types, TypeSet::of(&[CardType::Artifact, CardType::Creature]));
    assert!(c.subtypes.has(Subtype::Construct));
    assert!(c.has(Keyword::Haste));
    assert!(g.cards.get(c.card).prototyped, "its values are the prototyped face");
}

/// CR 718.3a, 718.4: in a hand it is its normal self; cast prototyped it costs exactly {2}{R} - not {1}{R}, not {2}{G}.
#[test]
fn a_prototyped_cast_costs_exactly_the_prototype_mana_cost() {
    let mut g = game();
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    assert_normal(&g, card);
    add(&mut g, Mana::R, 1);
    add(&mut g, Mana::C, 1);
    assert!(cast_option(&mut g, card, WAY_PROTOTYPE, false).is_none(), "{{1}}{{R}} does not pay {{2}}{{R}}");
    add(&mut g, Mana::G, 1);
    let mut g2 = game();
    let card2 = put(&mut g2, &AUTOMATON, Zone::hand(YOU));
    add(&mut g2, Mana::G, 1);
    add(&mut g2, Mana::C, 2);
    assert!(cast_option(&mut g2, card2, WAY_PROTOTYPE, false).is_none(), "{{2}}{{G}} does not pay {{2}}{{R}}");
    // {1}{R} and a third mana pay it; the normal cast's {7} is not offered.
    assert!(cast_option(&mut g, card, 0, false).is_none());
    let option = cast_option(&mut g, card, WAY_PROTOTYPE, false).expect("the prototyped cast is offered");
    let spell = {
        take(&mut g, &option);
        g.state.stack.last().expect("the spell").object
    };
    assert_eq!(g.state.player(YOU).pool.total(), 0, "it cost exactly {{2}}{{R}}");
    assert_prototyped(&g, spell);
}

/// CR 718.3b: the prototyped spell becomes a permanent with the prototype's characteristics; once it leaves the battlefield
/// it is its normal self again (CR 718.4, 400.7).
#[test]
fn a_prototyped_spell_resolves_into_a_prototyped_permanent_that_leaves_as_its_normal_self() {
    let mut g = game();
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    add(&mut g, Mana::R, 3);
    let spell = cast(&mut g, card, WAY_PROTOTYPE);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert_prototyped(&g, permanent);
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[permanent]);
    assert_eq!(gone.len(), 1);
    let card = g.state.obj(permanent).became.expect("it went to the graveyard");
    assert!(g.state.obj(card).zone.is(ZoneKind::Graveyard));
    assert_normal(&g, card);
}

/// CR 718.4: a prototyped spell that leaves the stack other than by resolving is its normal self where it goes.
#[test]
fn a_prototyped_spell_returned_to_hand_is_its_normal_self() {
    let mut g = game();
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    add(&mut g, Mana::R, 3);
    let spell = cast(&mut g, card, WAY_PROTOTYPE);
    let back = cantrip_api::Game::return_to_hand(&mut cx(&mut g), &[spell]);
    assert_eq!(back.len(), 1);
    assert!(g.state.obj(back[0]).zone.is(ZoneKind::Hand));
    assert_normal(&g, back[0]);
}

/// CR 718.4: cast normally, it is its normal self on the stack and on the battlefield.
#[test]
fn a_card_cast_normally_has_its_normal_characteristics() {
    let mut g = game();
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    add(&mut g, Mana::C, 7);
    let spell = cast(&mut g, card, 0);
    assert_normal(&g, spell);
    assert!(g.state.obj(spell).copied_card().is_none());
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_normal(&g, permanent);
}

/// CR 718.2a, 718.3d: a token copy of a prototyped permanent has the prototype's characteristics; one of a permanent cast
/// normally has the normal ones.
#[test]
fn a_copy_of_a_prototyped_permanent_is_prototyped() {
    let mut g = game();
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    add(&mut g, Mana::R, 3);
    let spell = cast(&mut g, card, WAY_PROTOTYPE);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    let values = cantrip_api::Query::copiable_values(&g, permanent);
    let tokens = cantrip_api::Game::create_token_copies_of(&mut cx(&mut g), values, YOU, 1).expect("nothing is asked");
    assert_eq!(tokens.len(), 1);
    assert_prototyped(&g, tokens[0]);
}

/// CR 718.3c: a copy of a prototyped spell is a prototyped spell.
#[test]
fn a_copy_of_a_prototyped_spell_is_prototyped() {
    let mut g = game();
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    add(&mut g, Mana::R, 3);
    let spell = cast(&mut g, card, WAY_PROTOTYPE);
    let copy = cantrip_api::Game::copy_spell(&mut cx(&mut g), spell, YOU, false).expect("nothing is asked").expect("a copy");
    assert_prototyped(&g, copy);
}

/// The ruling: casting it prototyped is no alternative cost, so "without paying its mana cost" may be applied to either cast.
#[test]
fn an_alternative_cost_applies_to_a_prototyped_cast() {
    let mut g = game();
    put(&mut g, &OMNISCIENT, Zone::BATTLEFIELD);
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    assert!(cast_option(&mut g, card, 0, true).is_some(), "the card itself, for nothing");
    let option = cast_option(&mut g, card, WAY_PROTOTYPE, true).expect("prototyped, for nothing");
    take(&mut g, &option);
    let spell = g.state.stack.last().expect("the spell").object;
    assert_prototyped(&g, spell);
}

/// The ruling: cast prototyped by an alternative cost, it is still a prototyped spell, and the permanent it becomes is
/// prototyped too (CR 718.3b).
#[test]
fn a_prototyped_spell_cast_for_an_alternative_cost_becomes_a_prototyped_permanent() {
    let mut g = game();
    put(&mut g, &OMNISCIENT, Zone::BATTLEFIELD);
    let card = put(&mut g, &AUTOMATON, Zone::hand(YOU));
    let option = cast_option(&mut g, card, WAY_PROTOTYPE, true).expect("prototyped, for nothing");
    take(&mut g, &option);
    let spell = g.state.stack.last().expect("the spell").object;
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert_prototyped(&g, permanent);
}

/// The ruling: prototype works from any zone the card could be cast from, and the cast is judged as the prototyped face
/// (CR 718.3a): a permission for permanent spells with mana value 3 or less casts it prototyped from the graveyard, for
/// {2}{R}, and not as its normal mana value 7 self.
#[test]
fn a_permission_judged_on_the_prototyped_face_casts_it_from_the_graveyard() {
    let mut g = game();
    put(&mut g, &RECURSION, Zone::BATTLEFIELD);
    let card = put(&mut g, &AUTOMATON, Zone::graveyard(YOU));
    add(&mut g, Mana::R, 3);
    add(&mut g, Mana::C, 4);
    let ways: Vec<u8> = options(&mut g).into_iter().filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).map(|o| o.index).collect();
    assert_eq!(ways, vec![WAY_PROTOTYPE], "only the prototyped face has mana value 3 or less");
    let spell = cast(&mut g, card, WAY_PROTOTYPE);
    assert_eq!(g.state.player(YOU).pool.total(), 4, "it cost {{2}}{{R}}");
    assert_prototyped(&g, spell);
}
