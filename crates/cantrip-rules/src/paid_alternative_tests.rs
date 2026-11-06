//! The alternative cost another object gives that a spell is cast for (`SpellAlternative`, CR 118.9, 601.2b) is recorded
//! with the cast (`Event::PaidAlternative`), so "once during each of your turns, you may cast ... without paying its mana
//! cost" (Zaffai and the Tempests; Vision, Spectral Synthezoid) can ask the history whether it has been used. Over test
//! cards: a split card with fuse cast fused from the hand for it (the fuse ruling, 2013-04-15, CR 702.102a), the once used
//! up by that cast, and a cast both permitted by one object and paid for by another's alternative cost, which records both.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind, WAY_FUSED};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Paid Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// An instant for {U} that does nothing.
static INSTANT: CardDef =
    CardDef { name: "Paid Test Instant", cost: mana("{U}"), has_cost: true, types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], nothing), ..CardDef::BLANK };

static LEFT: CardDef =
    CardDef { name: "Paid Test Left", cost: mana("{2}{U}"), has_cost: true, types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], nothing), ..CardDef::BLANK };
static RIGHT: CardDef =
    CardDef { name: "Paid Test Right", cost: mana("{2}{R}"), has_cost: true, types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], nothing), ..CardDef::BLANK };
/// A split instant with fuse (CR 702.102).
static FUSED: CardDef = CardDef {
    name: "Paid Test Left // Right",
    cost: mana("{4}{U}{R}"),
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    split: Some((&LEFT, &RIGHT)),
    fuse: true,
    ..CardDef::BLANK
};

/// "Once during each of your turns, you may cast an instant or sorcery spell from your hand without paying its mana cost":
/// used up by a spell cast for it this turn, as the history records (`Event::PaidAlternative`).
fn once_each_of_your_turns(q: &dyn Query, asker: &Asker, spell: ObjectId, player: PlayerId) -> bool {
    let c = q.chars(spell);
    player == asker.controller
        && q.state().turn.active == player
        && q.obj(spell).zone == Zone::hand(player)
        && (c.is(CardType::Instant) || c.is(CardType::Sorcery))
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::PaidAlternative { by, .. } if by.object == asker.source))
}

static TEMPEST: CardDef = CardDef {
    name: "Paid Test Tempest",
    types: TypeSet::of(&[CardType::Enchantment]),
    spell_alternatives: &[SpellAlternative {
        text: "Once during each of your turns",
        applies: once_each_of_your_turns,
        cost: InsteadOfManaCost::WithoutPaying,
        flash: false,
        kind: WayKind::Other,
    }],
    ..CardDef::BLANK
};

fn spells_you_cast(_: &dyn Query, asker: &Asker, _: ObjectId, player: PlayerId) -> bool {
    player == asker.controller
}
fn any_card(_: &dyn Query, _: &Asker, _: ObjectId, _: PlayerId) -> bool {
    true
}

/// "You may cast spells without paying their mana costs".
static SCROLL: CardDef = CardDef {
    name: "Paid Test Scroll",
    types: TypeSet::of(&[CardType::Enchantment]),
    spell_alternatives: &[SpellAlternative { text: "Without paying its mana cost", applies: spells_you_cast, cost: InsteadOfManaCost::WithoutPaying, flash: false, kind: WayKind::Other }],
    ..CardDef::BLANK
};
/// "You may cast spells from among cards in exile" (CR 601.3).
static EXILER: CardDef = CardDef {
    name: "Paid Test Exiler",
    types: TypeSet::of(&[CardType::Enchantment]),
    exile_casts: &[ExileCast { text: "Cast from exile", from: ZoneKind::Exile, allows: any_card, additional: Cost::NONE, any_type: false, any_color: false, lands: false, life_for_mana_value: false, instead: None }],
    ..CardDef::BLANK
};

/// "The next instant spell you cast this turn can be cast without paying its mana cost": a resolved ability's alternative cost
/// for the player's next spell (`EffectKind::SpellAlternative`), its source long gone from the battlefield. Asked of the
/// spell on the stack as well, to use it up, so it reads no zone.
fn instant_spells_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && q.chars(spell).is(CardType::Instant)
}
/// A sorcery for {U} that does nothing.
static SORCERY: CardDef =
    CardDef { name: "Paid Test Sorcery", cost: mana("{U}"), has_cost: true, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };
static GIFT: CardDef = CardDef {
    name: "Paid Test Gift",
    types: TypeSet::of(&[CardType::Sorcery]),
    lasting_spell_alternatives: &[SpellAlternative {
        text: "The next instant spell you cast this turn can be cast without paying its mana cost.",
        applies: instant_spells_you_cast,
        cost: InsteadOfManaCost::WithoutPaying,
        flash: false,
        kind: WayKind::Other,
    }],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Your main phase with priority, an empty stack and no mana.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &INSTANT, &LEFT, &RIGHT, &FUSED, &TEMPEST, &SCROLL, &EXILER, &GIFT, &SORCERY])));
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

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// The casts of `card` offered now: each one's way number, what permits it and what alternative cost pays for it.
fn casts(g: &mut Game, card: ObjectId) -> Vec<(u8, Option<ObjectId>, Option<ObjectId>)> {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    options
        .iter()
        .filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card))
        .map(|o| (o.index, o.granted_by.map(|g| g.object), o.paid_by.map(|g| g.object)))
        .collect()
}

/// Casts `card` by the offered option `way`, permitted by `by` and paid for by `paid`, and returns the spell.
fn cast(g: &mut Game, card: ObjectId, way: u8, paid: ObjectId) -> ObjectId {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == way && o.paid_by.is_some_and(|g| g.object == paid))
        .expect("the cast is offered")
        .clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("nothing to pay").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    g.state.stack.last().expect("the spell is on the stack").object
}

/// The records of the cast of `spell`: the alternative cost it was paid with, if any, and what permitted it.
fn recorded(g: &mut Game, spell: ObjectId) -> (Option<Grant>, Option<Grant>) {
    let cx = cx(g);
    let mut paid = None;
    let mut by = None;
    for r in cx.history().all() {
        match r.event {
            Event::PaidAlternative { spell: s, by: g } if s == spell => {
                assert!(by.is_none(), "recorded before its `Cast`");
                paid = Some(g);
            }
            Event::Cast(c) if c.spell == spell => by = Some(c.by),
            _ => {}
        }
    }
    (paid, by.expect("the cast is recorded").some())
}

/// The fuse ruling (CR 702.102a, 118.9): a split card with fuse cast from the hand for an alternative cost another object
/// gives may be cast fused, both halves for it. The cast records that alternative cost, the once is used up by it, and
/// the next instant is offered nothing (it costs {U}, and there is no mana).
#[test]
fn a_fused_cast_for_a_once_each_turn_alternative_records_it_and_uses_it_up() {
    let mut g = game();
    let tempest = put(&mut g, &TEMPEST, Zone::BATTLEFIELD);
    let fused = put(&mut g, &FUSED, Zone::hand(YOU));
    let instant = put(&mut g, &INSTANT, Zone::hand(YOU));
    let offered = casts(&mut g, fused);
    assert!(offered.contains(&(WAY_FUSED, None, Some(tempest))), "{offered:?}");
    assert_eq!(casts(&mut g, instant), [(0, None, Some(tempest))]);
    let spell = cast(&mut g, fused, WAY_FUSED, tempest);
    let (paid, by) = recorded(&mut g, spell);
    assert_eq!(paid.map(|g| g.object), Some(tempest));
    assert_eq!(by, None);
    assert!(casts(&mut g, instant).is_empty());
}

/// A spell cast for its mana cost records no alternative cost, and leaves the once unused.
#[test]
fn a_cast_for_its_mana_cost_records_no_alternative() {
    let mut g = game();
    let tempest = put(&mut g, &TEMPEST, Zone::BATTLEFIELD);
    let first = put(&mut g, &INSTANT, Zone::hand(YOU));
    let second = put(&mut g, &INSTANT, Zone::hand(YOU));
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::U, 1, source, None, ManaLasts::Step, false);
    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority") };
    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(first) && o.paid_by.is_none()).expect("offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(&mut g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    let spell = g.state.stack.last().expect("on the stack").object;
    assert_eq!(recorded(&mut g, spell), (None, None));
    assert_eq!(casts(&mut g, second), [(0, None, Some(tempest))]);
}

/// CR 601.3, 118.9: a card cast from exile by one permanent's permission and for another's alternative cost records both:
/// the permission as the cast's `by`, the alternative cost beside it.
#[test]
fn a_cast_permitted_by_one_object_and_paid_for_by_another_records_both() {
    let mut g = game();
    let scroll = put(&mut g, &SCROLL, Zone::BATTLEFIELD);
    let exiler = put(&mut g, &EXILER, Zone::BATTLEFIELD);
    let card = put(&mut g, &INSTANT, Zone::exile(YOU));
    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority") };
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.granted_by.is_some_and(|g| g.object == exiler) && o.paid_by.is_some_and(|g| g.object == scroll))
        .expect("offered")
        .clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(&mut g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("nothing to pay").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    let spell = g.state.stack.last().expect("on the stack").object;
    let (paid, by) = recorded(&mut g, spell);
    assert_eq!(paid.map(|g| g.object), Some(scroll));
    assert_eq!(by.map(|g| g.object), Some(exiler));
}

/// The effect `player` has from `gift`, a resolved ability's: "the next spell you cast ... without paying its mana cost".
fn give(g: &mut Game, gift: ObjectId, player: PlayerId) {
    let kind = EffectKind::SpellAlternative { card: g.cards.id_of(&GIFT), index: 0, player, words: Words::NONE };
    cantrip_api::Game::add_player_effect(&mut cx(g), player, gift, kind, Duration::EndOfTurn);
}

fn lasting_effects(g: &Game) -> usize {
    g.state.effects.iter().filter(|e| matches!(e.kind, EffectKind::SpellAlternative { .. })).count()
}

/// CR 611.2c, 118.9, 601.2b: an alternative cost a resolved ability gave the next spell is offered for the spells it applies to
/// whether or not its source is still anywhere, is recorded with the cast, and is used up by that cast: the next spell is
/// offered nothing (it costs {U}, and there is no mana).
#[test]
fn a_lasting_alternative_is_offered_recorded_and_used_up_by_the_next_spell() {
    let mut g = game();
    let gift = put(&mut g, &GIFT, Zone::graveyard(YOU));
    give(&mut g, gift, YOU);
    let first = put(&mut g, &INSTANT, Zone::hand(YOU));
    let second = put(&mut g, &INSTANT, Zone::hand(YOU));
    assert_eq!(casts(&mut g, first), [(0, None, Some(gift))]);
    let spell = cast(&mut g, first, 0, gift);
    let (paid, by) = recorded(&mut g, spell);
    assert_eq!(paid.map(|g| g.object), Some(gift));
    assert_eq!(by, None);
    assert_eq!(lasting_effects(&g), 0);
    assert!(casts(&mut g, second).is_empty());
}

/// "The next instant spell" is used up by the first it applies to even when that one is paid for with mana (CR 611.2a), and a
/// spell it does not apply to leaves it: a sorcery paid for does not use it up.
#[test]
fn a_lasting_alternative_is_used_up_by_a_spell_paid_for_and_not_by_one_it_is_not_for() {
    let mut g = game();
    let gift = put(&mut g, &GIFT, Zone::graveyard(YOU));
    give(&mut g, gift, YOU);
    let sorcery = put(&mut g, &SORCERY, Zone::hand(YOU));
    let first = put(&mut g, &INSTANT, Zone::hand(YOU));
    let second = put(&mut g, &INSTANT, Zone::hand(YOU));
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::U, 3, source, None, ManaLasts::Step, false);
    let pay = |g: &mut Game, card: ObjectId| {
        let r = next(g);
        let RequestKind::Priority { options } = &r.kind else { panic!("expected priority") };
        let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.paid_by.is_none()).expect("offered").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
        loop {
            let r = next(g);
            let RequestKind::PayCosts { options, .. } = &r.kind else { break };
            let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
            g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
        }
    };
    pay(&mut g, sorcery);
    assert_eq!(lasting_effects(&g), 1, "a spell it is not for leaves it");
    assert_eq!(casts(&mut g, first), [(0, None, None), (0, None, Some(gift))], "an instant is offered it, beside its mana cost");
    pay(&mut g, first);
    assert_eq!(lasting_effects(&g), 0, "a spell it is for uses it up, paid for or not");
    assert_eq!(casts(&mut g, second), [(0, None, None)], "nothing but its mana cost is left");
}

/// It is the player's: another player's spell is offered nothing for it and does not use it up (CR 109.5).
#[test]
fn a_lasting_alternative_is_for_its_players_spells_only() {
    let mut g = game();
    let gift = put(&mut g, &GIFT, Zone::graveyard(YOU));
    give(&mut g, gift, PlayerId(1));
    let card = put(&mut g, &INSTANT, Zone::hand(YOU));
    assert!(casts(&mut g, card).is_empty());
    assert_eq!(lasting_effects(&g), 1);
}
