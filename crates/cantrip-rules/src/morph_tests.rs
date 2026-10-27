//! Morph, megamorph and disguise as the core does them (CR 702.37, 702.168, 708), over test cards: a card cast face down for
//! {3} is a 2/2 face-down creature spell with no text, no name, no subtypes and no mana cost (with ward {2} for disguise),
//! which only its controller may look at (CR 708.5), resolves into a face-down permanent with the same values, and is
//! revealed if it goes anywhere else (CR 708.9); a face-down permanent that would have morph or disguise face up is turned
//! face up for that cost as a special action any time its controller has priority (CR 702.37e, 702.168d, 116.2b), whatever
//! put it there (CR 701.40c); megamorph's +1/+1 counter only when its megamorph cost turned it up (CR 702.37b, 708.11).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup, resolve};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Morph Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Flying / Morph {3}{W}": a 4/5 Bird for {4}{W}.
static SKYWARDEN: CardDef = CardDef {
    name: "Morph Test Skywarden",
    cost: mana("{4}{W}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bird]),
    keywords: Keywords::of(&[Keyword::Flying]),
    power: Some(4),
    toughness: Some(5),
    text: "Flying\nMorph {3}{W}",
    ways: &[face_down_way(WayKind::Morph, Cost::mana(mana("{3}{W}")), "Morph {3}{W}")],
    ..CardDef::BLANK
};

/// "Megamorph {2}": a 3/3 for {1}{G}{G}.
static GROWER: CardDef = CardDef {
    name: "Morph Test Grower",
    cost: mana("{1}{G}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    text: "Megamorph {2}",
    ways: &[face_down_way(WayKind::Megamorph, Cost::mana(mana("{2}")), "Megamorph {2}")],
    ..CardDef::BLANK
};

/// "Disguise {1}{U}": a 3/2 for {2}{U}.
static MASK: CardDef = CardDef {
    name: "Morph Test Mask",
    cost: mana("{2}{U}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(2),
    text: "Disguise {1}{U}",
    ways: &[face_down_way(WayKind::Disguise, Cost::mana(mana("{1}{U}")), "Disguise {1}{U}")],
    ..CardDef::BLANK
};

/// "Morph—Pay 5 life.": a 3/4 for {3}{B}.
static CUTTHROAT: CardDef = CardDef {
    name: "Morph Test Cutthroat",
    cost: mana("{3}{B}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(4),
    text: "Morph\u{2014}Pay 5 life.",
    ways: &[face_down_way(WayKind::Morph, Cost { life: 5, ..Cost::NONE }, "Morph\u{2014}Pay 5 life.")],
    ..CardDef::BLANK
};

/// "Morph {X}{G}": a 2/2 for {1}{G}.
static SWELLER: CardDef = CardDef {
    name: "Morph Test Sweller",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    text: "Morph {X}{G}",
    ways: &[face_down_way(WayKind::Morph, Cost::mana(mana("{X}{G}")), "Morph {X}{G}")],
    ..CardDef::BLANK
};

/// "You may cast Bird spells as though they had flash" (Rattlechains' kind of permission, CR 601.3, 702.8a).
fn bird_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && q.chars(spell).has_subtype(Subtype::Bird)
}
static BIRD_FLASH: CardDef = CardDef {
    name: "Morph Test Bird Flash",
    types: TypeSet::of(&[CardType::Enchantment]),
    flash_rules: &[bird_you_cast],
    ..CardDef::BLANK
};

/// "You may cast creature spells as though they had flash" (Leyline of Anticipation's kind, CR 601.3, 702.8a).
fn creature_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && q.chars(spell).is(CardType::Creature)
}
static CREATURE_FLASH: CardDef = CardDef {
    name: "Morph Test Creature Flash",
    types: TypeSet::of(&[CardType::Enchantment]),
    flash_rules: &[creature_you_cast],
    ..CardDef::BLANK
};

/// "You may cast colorless nonland spells from the top of your library" (Mystic Forge's, CR 601.3).
fn colorless(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    q.chars(card).colors.is_empty()
}
static COLORLESS_FROM_TOP: CardDef = CardDef {
    name: "Morph Test Colorless From Top",
    types: TypeSet::of(&[CardType::Artifact]),
    play_rules: &[PlayRule::LibraryTopSpells(colorless)],
    ..CardDef::BLANK
};

/// "You may cast white spells from the top of your library".
fn white(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    q.chars(card).colors.has(Color::White)
}
static WHITE_FROM_TOP: CardDef = CardDef {
    name: "Morph Test White From Top",
    types: TypeSet::of(&[CardType::Artifact]),
    play_rules: &[PlayRule::LibraryTopSpells(white)],
    ..CardDef::BLANK
};

/// "White spells can't be cast."
fn no_white(q: &dyn Query, _: &Asker, _: PlayerId, spell: ObjectId) -> bool {
    !q.chars(spell).colors.has(Color::White)
}
static NO_WHITE: CardDef = CardDef { name: "Morph Test No White", types: TypeSet::of(&[CardType::Enchantment]), spell_rules: &[no_white], ..CardDef::BLANK };

/// "Colorless spells can't be cast."
fn no_colorless(q: &dyn Query, _: &Asker, _: PlayerId, spell: ObjectId) -> bool {
    !q.chars(spell).colors.is_empty()
}
static NO_COLORLESS: CardDef =
    CardDef { name: "Morph Test No Colorless", types: TypeSet::of(&[CardType::Enchantment]), spell_rules: &[no_colorless], ..CardDef::BLANK };

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A sorcery whose resolution stands in for an effect that lets you cast a card (CR 608.2g).
static CASTER: CardDef = CardDef { name: "Morph Test Caster", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// Your main phase with priority and an empty stack.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &SKYWARDEN, &GROWER, &MASK, &CUTTHROAT, &SWELLER, &BIRD_FLASH, &CREATURE_FLASH, &COLORLESS_FROM_TOP, &WHITE_FROM_TOP, &NO_WHITE, &NO_COLORLESS, &CASTER])));
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

/// `n` of `mana` in your pool, from nothing in particular; a priority already offered is offered again, with it.
fn add(g: &mut Game, mana: Mana, n: u32) {
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, mana, n, source, None, false);
    reoffer(g);
}

/// Has the waiting priority offered again, of the position as the test changed it.
fn reoffer(g: &mut Game) {
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

/// The option to cast `card` face down by its first way (morph, megamorph or disguise), if it is offered.
fn face_down_cast(g: &mut Game, card: ObjectId) -> Option<ActionOption> {
    options(g).into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == 1)
}

/// The option to turn `permanent` face up `by` that special action, if it is offered.
fn turn_up_option(g: &mut Game, permanent: ObjectId, by: TurnUp) -> Option<ActionOption> {
    let chars = g.chars(permanent);
    options(g).into_iter().find(|o| {
        o.kind == ActionKind::ActivateAbility && o.source == Some(permanent) && chars.abilities().nth(o.index as usize).and_then(|a| g.cards.turn_up(a)) == Some(by)
    })
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

/// Casts `card` face down, paying {3} from the pool; the spell.
fn cast_face_down(g: &mut Game, card: ObjectId) -> ObjectId {
    let option = face_down_cast(g, card).expect("casting it face down is offered");
    take(g, &option);
    let spell = g.state.stack.last().expect("the spell is on the stack").object;
    assert_eq!(g.state.obj(card).became, Some(spell));
    spell
}

/// Passes priority until the stack is empty.
fn resolve_all(g: &mut Game) {
    while !g.state.stack.is_empty() {
        let r = next(g);
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "unexpected {:?}", r.kind);
        g.answer(r.id, r.player, Answer::Pass).expect("a legal answer");
    }
    g.run();
}

/// What a face-down spell or permanent is (CR 708.2a): a nameless 2/2 creature with no subtypes, no color, no keywords
/// and no mana cost.
fn assert_face_down_values(g: &Game, id: ObjectId) {
    let c = g.chars(id);
    assert!(g.state.obj(id).face_down);
    assert_eq!(c.name, "");
    assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    assert!(c.is(CardType::Creature));
    assert_eq!(c.types, TypeSet::of(&[CardType::Creature]));
    assert_eq!(c.subtypes, SubtypeSet::NONE);
    assert!(c.colors.is_empty());
    assert_eq!(c.keywords, Keywords::NONE);
    assert_eq!(Query::mana_value(g, id), 0);
}

fn has_ward(g: &Game, id: ObjectId) -> bool {
    g.chars(id).abilities().any(|a| matches!(g.cards.ability(a), AbilityDef::Triggered(t) if t.ward))
}

/// CR 702.37a, 702.37c, 708.4: cast for {3} rather than its mana cost, two mana does not pay it; on the stack it is a 2/2
/// face-down creature spell with no text, no name, no subtypes and no mana cost, under its caster.
#[test]
fn a_card_cast_by_morph_is_a_nameless_two_two_face_down_creature_spell_for_three() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 2);
    assert!(face_down_cast(&mut g, card).is_none(), "{{2}} does not pay {{3}}");
    add(&mut g, Mana::C, 1);
    let spell = cast_face_down(&mut g, card);
    assert_eq!(g.state.player(YOU).pool.total(), 0, "it cost exactly {{3}}");
    assert!(g.state.obj(spell).zone.is(ZoneKind::Stack));
    assert_eq!(g.state.obj(spell).controller, YOU);
    assert_face_down_values(&g, spell);
    assert_eq!(g.chars(spell).card, g.cards.id_of(&intrinsic::MORPHED));
    // CR 708.7: a spell can't be turned face up.
    add(&mut g, Mana::W, 4);
    assert!(turn_up_option(&mut g, spell, TurnUp::Morph).is_none());
}

/// CR 708.5: you may look at a face-down spell you control; an opponent may not.
#[test]
fn only_its_controller_may_look_at_a_face_down_spell() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    assert!(!crate::hidden_from(&g.state, YOU, spell));
    assert!(crate::hidden_from(&g.state, THEM, spell));
}

/// CR 702.37c, 708.3: it resolves into a face-down permanent with the same characteristics, which entered face down.
#[test]
fn a_face_down_spell_resolves_into_a_face_down_permanent() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert_face_down_values(&g, permanent);
    assert!(!crate::hidden_from(&g.state, YOU, permanent));
    assert!(crate::hidden_from(&g.state, THEM, permanent));
    assert!(g.log.iter().any(|r| matches!(r.event, Event::EnteredFaceDown { permanent: p, .. } if p == permanent)));
}

/// CR 702.37e: turned face up for exactly its morph cost, {3}{W} - not for {2}{W} - as a special action of its controller's
/// with priority, it is the card it is (CR 708.8).
#[test]
fn a_morph_is_turned_face_up_for_its_morph_cost_by_its_controller() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    add(&mut g, Mana::W, 1);
    add(&mut g, Mana::C, 2);
    assert!(turn_up_option(&mut g, permanent, TurnUp::Morph).is_none(), "{{2}}{{W}} does not pay {{3}}{{W}}");
    // Never for its mana cost: it was cast face down, not manifested (CR 701.40b).
    add(&mut g, Mana::W, 4);
    assert!(turn_up_option(&mut g, permanent, TurnUp::ManaCost).is_none());
    // Only its controller, with priority, may (CR 702.37e).
    assert!(!priority_options(&mut cx(&mut g), THEM).iter().any(|o| o.source == Some(permanent)));
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    add(&mut g, Mana::W, 1);
    add(&mut g, Mana::C, 3);
    let option = turn_up_option(&mut g, permanent, TurnUp::Morph).expect("turning it face up is offered");
    let r = take(&mut g, &option);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "a special action leaves its player priority");
    assert!(g.state.stack.is_empty(), "a special action doesn't use the stack");
    assert_eq!(g.state.player(YOU).pool.total(), 0, "it cost exactly {{3}}{{W}}");
    assert!(!g.state.obj(permanent).face_down);
    let c = g.chars(permanent);
    assert_eq!((c.name, c.power, c.toughness), ("Morph Test Skywarden", Some(4), Some(5)));
    assert!(c.has(Keyword::Flying));
    assert_eq!(g.state.obj(permanent).counters(CounterKind::PlusOne), 0, "morph puts no counter");
    assert!(g.log.iter().any(|r| matches!(r.event, Event::TurnedFaceUp { permanent: p, player } if p == permanent && player == YOU)));
}

/// CR 702.37b, 708.11: turned face up for its megamorph cost it gets a +1/+1 counter as it is turned face up.
#[test]
fn a_megamorph_turned_up_for_its_megamorph_cost_gets_a_counter() {
    let mut g = game();
    let card = put(&mut g, &GROWER, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    add(&mut g, Mana::C, 2);
    let option = turn_up_option(&mut g, permanent, TurnUp::Morph).expect("turning it face up is offered");
    take(&mut g, &option);
    assert!(!g.state.obj(permanent).face_down);
    assert_eq!(g.state.obj(permanent).counters(CounterKind::PlusOne), 1);
    assert_eq!((g.chars(permanent).power, g.chars(permanent).toughness), (Some(4), Some(4)));
    // The counter is on it as it is turned face up (CR 708.11): put before the turning up is recorded.
    let put_at = g.log.iter().position(|r| matches!(r.event, Event::CountersPut { object, .. } if object == permanent)).expect("the counter");
    let up_at = g.log.iter().position(|r| matches!(r.event, Event::TurnedFaceUp { permanent: p, .. } if p == permanent)).expect("turned up");
    assert!(put_at < up_at);
}

/// CR 701.40b-c, 702.37b: a manifested megamorph card turned face up for its mana cost gets no counter; turned face up for
/// its megamorph cost, it does.
#[test]
fn a_manifested_megamorph_gets_its_counter_only_when_its_megamorph_cost_turns_it_up() {
    for (by, pool, counters) in [(TurnUp::ManaCost, &[Mana::C, Mana::G, Mana::G][..], 0), (TurnUp::Morph, &[Mana::C, Mana::C][..], 1)] {
        let mut g = game();
        let card = put(&mut g, &GROWER, Zone::library(YOU));
        let manifested = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false)
            .expect("nothing is asked")
            .expect("it enters face down");
        for &m in pool {
            add(&mut g, m, 1);
        }
        let option = turn_up_option(&mut g, manifested, by).expect("turning it face up is offered");
        take(&mut g, &option);
        assert!(!g.state.obj(manifested).face_down);
        assert_eq!(g.state.obj(manifested).counters(CounterKind::PlusOne), counters, "turned up by {by:?}");
        assert_eq!(g.state.player(YOU).pool.total(), 0);
    }
}

/// CR 701.40c: a manifested card with morph may be turned face up for its morph cost as well as for its mana cost.
#[test]
fn a_manifested_card_with_morph_is_turned_face_up_for_its_morph_cost() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::library(YOU));
    let manifested = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false)
        .expect("nothing is asked")
        .expect("it enters face down");
    add(&mut g, Mana::W, 1);
    add(&mut g, Mana::C, 3);
    // {3}{W} pays its morph cost and not its mana cost, {4}{W}.
    assert!(turn_up_option(&mut g, manifested, TurnUp::ManaCost).is_none());
    let option = turn_up_option(&mut g, manifested, TurnUp::Morph).expect("its morph cost is offered");
    take(&mut g, &option);
    assert!(!g.state.obj(manifested).face_down);
    assert_eq!(g.chars(manifested).name, "Morph Test Skywarden");
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

/// CR 702.168a-b, 702.168d: cast by disguise it is a 2/2 with ward {2} on the stack and on the battlefield, and turned face
/// up for its disguise cost it has no ward.
#[test]
fn a_disguised_card_has_ward_two_face_down_and_none_face_up() {
    let mut g = game();
    let card = put(&mut g, &MASK, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    assert_face_down_values(&g, spell);
    assert!(has_ward(&g, spell));
    assert_eq!(g.chars(spell).card, g.cards.id_of(&intrinsic::DISGUISED));
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert_face_down_values(&g, permanent);
    assert!(has_ward(&g, permanent));
    add(&mut g, Mana::U, 1);
    assert!(turn_up_option(&mut g, permanent, TurnUp::Disguise).is_none(), "{{U}} does not pay {{1}}{{U}}");
    add(&mut g, Mana::C, 1);
    let option = turn_up_option(&mut g, permanent, TurnUp::Disguise).expect("its disguise cost is offered");
    take(&mut g, &option);
    assert!(!g.state.obj(permanent).face_down);
    assert_eq!(g.chars(permanent).name, "Morph Test Mask");
    assert!(!has_ward(&g, permanent));
}

/// CR 708.9: a face-down spell countered goes to its owner's graveyard face up, revealed to all players.
#[test]
fn a_countered_face_down_spell_is_revealed_and_goes_to_the_graveyard() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve::counter(&mut cx(&mut g), spell);
    let gone = g.state.obj(spell).became.expect("it left the stack");
    assert!(g.state.obj(gone).zone.is(ZoneKind::Graveyard));
    assert!(!g.state.obj(gone).face_down);
    assert_eq!(g.chars(gone).name, "Morph Test Skywarden");
    assert!(!crate::hidden_from(&g.state, THEM, gone));
    assert!(g.log.iter().any(|r| matches!(r.event, Event::Revealed { player, card } if player == YOU && card == gone)));
}

/// CR 702.37e, 118.3: a morph cost that is not mana - "Pay 5 life" - is paid as a cost is, and not offered to a player who
/// can't pay it.
#[test]
fn a_morph_cost_of_life_is_paid_in_life() {
    let mut g = game();
    let card = put(&mut g, &CUTTHROAT, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    g.state.update_player(YOU, |p| p.life = 4);
    reoffer(&mut g);
    assert!(turn_up_option(&mut g, permanent, TurnUp::Morph).is_none(), "4 life does not pay 5");
    g.state.update_player(YOU, |p| p.life = 20);
    reoffer(&mut g);
    let option = turn_up_option(&mut g, permanent, TurnUp::Morph).expect("its morph cost is offered");
    take(&mut g, &option);
    assert!(!g.state.obj(permanent).face_down);
    assert_eq!(g.state.player(YOU).life, 15);
}

/// CR 707.2, 707.10, 708.2: a copy of a face-down spell (Double Major's) copies the face-down values and not the face-down
/// status: a face-up nameless 2/2 spell every player sees as that, which becomes a face-up nameless 2/2 token that can't be
/// turned face up (the Clone example of CR 707.2) - never a copy of the card behind it.
#[test]
fn a_copy_of_a_face_down_spell_is_a_face_up_nameless_two_two() {
    let mut g = game();
    let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    let copy = crate::ops::copy_spell(&mut cx(&mut g), spell, THEM, false).expect("nothing is asked").expect("it is copied");
    assert!(!g.state.obj(copy).face_down);
    assert!(!crate::hidden_from(&g.state, YOU, copy) && !crate::hidden_from(&g.state, THEM, copy));
    let c = g.chars(copy);
    assert_eq!((c.name, c.power, c.toughness), ("", Some(2), Some(2)));
    reoffer(&mut g);
    resolve_all(&mut g);
    let theirs: Vec<ObjectId> = g.state.zone(Zone::BATTLEFIELD).iter().copied().filter(|&id| g.state.obj(id).controller == THEM).collect();
    let [token] = theirs[..] else { panic!("one token: {theirs:?}") };
    assert!(g.state.obj(token).token && !g.state.obj(token).face_down);
    let c = g.chars(token);
    assert_eq!((c.name, c.power, c.toughness), ("", Some(2), Some(2)));
    assert_eq!(c.keywords, Keywords::NONE);
    // It is face up: no morph cost turns it up (CR 707.2's Clone).
    g.state.update_turn(|t| t.priority = THEM);
    assert!(!priority_options(&mut cx(&mut g), THEM).iter().any(|o| o.source == Some(token) && o.kind == ActionKind::ActivateAbility));
}

/// CR 702.37e-f, 107.3a: an X in a morph cost is announced as the special action is taken, paid as that much generic mana,
/// and kept on the permanent for its other abilities to read.
#[test]
fn an_x_in_a_morph_cost_is_announced_paid_and_kept() {
    let mut g = game();
    let card = put(&mut g, &SWELLER, Zone::hand(YOU));
    add(&mut g, Mana::C, 3);
    let spell = cast_face_down(&mut g, card);
    resolve_all(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    add(&mut g, Mana::G, 1);
    add(&mut g, Mana::C, 3);
    let option = turn_up_option(&mut g, permanent, TurnUp::Morph).expect("turning it face up is offered");
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    let mut announced = false;
    loop {
        let r = next(&mut g);
        let RequestKind::PayCosts { options, announcing, .. } = &r.kind else { break };
        if *announcing {
            g.answer(r.id, r.player, Answer::Announce(2)).expect("X = 2 is legal");
            announced = true;
            continue;
        }
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    assert!(announced, "X is announced");
    assert!(!g.state.obj(permanent).face_down);
    assert_eq!(g.state.player(YOU).pool.total(), 1, "X = 2 paid {{2}}{{G}} of the four");
    assert_eq!(g.state.obj(permanent).x, 2, "the X chosen is kept (CR 702.37f)");
}

/// CR 708.9: a face-down permanent - cast face down or manifested - that leaves the battlefield is revealed by its owner as
/// it moves, and arrives face up.
#[test]
fn a_face_down_permanent_that_leaves_the_battlefield_is_revealed() {
    for manifest in [false, true] {
        let mut g = game();
        let permanent = if manifest {
            let card = put(&mut g, &SKYWARDEN, Zone::library(YOU));
            cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false)
                .expect("nothing is asked")
                .expect("it enters face down")
        } else {
            let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
            add(&mut g, Mana::C, 3);
            let spell = cast_face_down(&mut g, card);
            resolve_all(&mut g);
            g.state.obj(spell).became.expect("it resolved")
        };
        crate::ops::move_to(&mut cx(&mut g), permanent, Zone::graveyard(YOU));
        let gone = g.state.obj(permanent).became.expect("it left the battlefield");
        assert!(!g.state.obj(gone).face_down, "manifested: {manifest}");
        assert!(!crate::hidden_from(&g.state, THEM, gone));
        assert!(g.log.iter().any(|r| matches!(r.event, Event::Revealed { player, card } if player == YOU && card == gone)), "manifested: {manifest}");
    }
}

/// CR 702.37c, 708.4, 307.1: cast face down it is a creature spell with no text, cast when a sorcery could be: not with
/// another spell on the stack. A permission to cast spells as though they had flash is judged on the face-down spell
/// (CR 601.3e): one for Bird spells, which the card is, gives it none - the card's own cast still has it - and one for
/// creature spells does.
#[test]
fn a_face_down_cast_has_the_speed_of_the_face_down_spell() {
    // (permission, the face-down cast offered with a spell on the stack, the card's own cast offered then)
    for (permission, face_down, own) in [(None, false, false), (Some(&BIRD_FLASH), false, true), (Some(&CREATURE_FLASH), true, true)] {
        let mut g = game();
        if let Some(p) = permission {
            put(&mut g, p, Zone::BATTLEFIELD);
        }
        let first = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
        let second = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
        add(&mut g, Mana::C, 3);
        cast_face_down(&mut g, first);
        add(&mut g, Mana::C, 4);
        add(&mut g, Mana::W, 1);
        let offered = options(&mut g);
        let cast = |index: u8| offered.iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(second) && o.index == index);
        assert_eq!(cast(1), face_down, "face down, {:?}", permission.map(|p| p.name));
        assert_eq!(cast(0), own, "its own cast, {:?}", permission.map(|p| p.name));
    }
}

/// CR 601.3e, 702.37c, 708.4: a permission to cast spells of some kind is judged on the face-down spell a morph casts: Mystic
/// Forge's "colorless nonland spells from the top of your library" lets the white card on top be cast face down, a
/// colorless 2/2, and not for its mana cost; one for white spells the other way about.
#[test]
fn a_permission_is_judged_on_the_face_down_spell() {
    // (permission, the face-down cast offered, the card's own cast offered)
    for (permission, face_down, own) in [(&COLORLESS_FROM_TOP, true, false), (&WHITE_FROM_TOP, false, true)] {
        let mut g = game();
        put(&mut g, permission, Zone::BATTLEFIELD);
        let card = g.state.push_obj(Obj::new(g.cards.id_of(&SKYWARDEN), YOU, Zone::library(YOU)));
        g.state.zone_insert(Zone::library(YOU), 0, card);
        add(&mut g, Mana::W, 1);
        add(&mut g, Mana::C, 4);
        let offered = options(&mut g);
        let cast = |index: u8| offered.iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == index);
        assert_eq!(cast(1), face_down, "face down, under {}", permission.name);
        assert_eq!(cast(0), own, "for its mana cost, under {}", permission.name);
    }
}

/// CR 101.2, 601.3e, 708.4: a ban on casting spells is judged on the spell each way would cast, as an effect resolving lets
/// you cast the card (CR 608.2g): "white spells can't be cast" leaves the white card its face-down cast, a colorless spell,
/// and "colorless spells can't be cast" its own; both leave it none.
#[test]
fn a_ban_judges_the_face_down_spell_apart_during_resolution() {
    for (bans, castable) in [(&[&NO_WHITE][..], true), (&[&NO_COLORLESS][..], true), (&[&NO_WHITE, &NO_COLORLESS][..], false)] {
        let mut g = game();
        for &ban in bans {
            put(&mut g, ban, Zone::BATTLEFIELD);
        }
        let card = put(&mut g, &SKYWARDEN, Zone::hand(YOU));
        let caster = put(&mut g, &CASTER, Zone::STACK);
        let ability = AbilityRef { card: g.cards.id_of(&CASTER), index: 0 };
        g.state.stack_push(crate::actions::stack_item(StackKind::Spell, caster, ability, YOU, &[]));
        let serial = g.state.stack.last().expect("the caster").serial;
        g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
        let frame = g.state.agenda.len() - 1;
        let mut resolving = Cx { frame, ..cx(&mut g) };
        let ways = crate::actions::cast_ways_during_resolution(&mut resolving, card, YOU);
        assert_eq!(!ways.is_empty(), castable, "{:?}", bans.iter().map(|b| b.name).collect::<Vec<_>>());
    }
}
