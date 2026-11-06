//! Replacement bookkeeping belongs to the rules action (CR 614.6, 615.7, 616.1).
//! Callback instructions still state their own boundaries (CR 603.8, 608.2c).
use crate::{Cx, Game, Setup, StateCheck};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static BEAR: CardDef = CardDef {
    name: "Replacement boundary creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};
fn destruction(_: &dyn Query, _: &Asker, affected: ObjectId, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.object == affected && z.destruction())
}
fn cancel(_: &dyn Query, _: &Asker, _: ObjectId, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p {
        z.replaced = true;
    }
}
fn heal(g: &mut dyn cantrip_api::Game, _: &Asker, _: ObjectId, before: &Proposal, _: &Proposal) {
    if let Proposal::ZoneChange(z) = before {
        g.heal(z.object);
    }
}
fn damage_to_you(_: &dyn Query, a: &Asker, _: ObjectId, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Player(a.controller) && d.amount > 0)
}
fn unchanged(_: &dyn Query, _: &Asker, _: ObjectId, _: &mut Proposal) {}
fn counters_for(_: &dyn Query, _: &Asker, affected: ObjectId, p: &Proposal) -> bool {
    matches!(p, Proposal::Counters(c) if c.object == affected && c.counts[CounterKind::PlusOne as usize] > 0)
}
fn one_more_counter(_: &dyn Query, _: &Asker, _: ObjectId, p: &mut Proposal) {
    if let Proposal::Counters(c) = p {
        c.counts[CounterKind::PlusOne as usize] += 1;
    }
}
fn reflect(g: &mut dyn cantrip_api::Game, a: &Asker, _: ObjectId, before: &Proposal, _: &Proposal) {
    if let Proposal::Damage(d) = before {
        let to = g.controller(d.source);
        g.damage(a.source, Target::Player(to), d.amount);
    }
}
static GRANTER: CardDef = CardDef {
    name: "Replacement boundary grant",
    types: TypeSet::of(&[CardType::Instant]),
    granted_replacements: &[
        granted_replacement(
            ReplacementKind::ZoneChange,
            destruction,
            cancel,
            "Remove damage instead.",
        )
        .once()
        .then(heal),
        granted_replacement(
            ReplacementKind::Damage,
            damage_to_you,
            unchanged,
            "The next damage also damages its controller.",
        )
        .once()
        .then(reflect),
        granted_replacement(
            ReplacementKind::Damage,
            damage_to_you,
            unchanged,
            "An unchanged once-only replacement.",
        )
        .once(),
        granted_replacement(
            ReplacementKind::Counters,
            counters_for,
            one_more_counter,
            "One additional counter next time.",
        )
        .once(),
    ],
    granted_preventions: &[prevention::ANY_DAMAGE],
    ..CardDef::BLANK
};
fn printed_damage(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Player(YOU) && d.amount > 0)
}
fn printed_unchanged(_: &dyn Query, _: &Asker, _: &mut Proposal) {}
fn two_effects(g: &mut dyn cantrip_api::Game, a: &Asker, _: &Proposal, _: &Proposal) {
    g.add_effect(
        a.source,
        a.source,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
        Duration::EndOfTurn,
    );
    g.add_effect(
        a.source,
        a.source,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])),
        Duration::EndOfTurn,
    );
}
fn grouped_effects(g: &mut dyn cantrip_api::Game, a: &Asker, b: &Proposal, p: &Proposal) {
    cantrip_api::instruction(g, |g| two_effects(g, a, b, p));
}
fn separated_effects(g: &mut dyn cantrip_api::Game, a: &Asker, _: &Proposal, _: &Proposal) {
    g.add_effect(
        a.source,
        a.source,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
        Duration::EndOfTurn,
    );
    g.end_sentence();
    g.add_effect(
        a.source,
        a.source,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])),
        Duration::EndOfTurn,
    );
}
fn leading_boundary(g: &mut dyn cantrip_api::Game, a: &Asker, before: &Proposal, after: &Proposal) {
    g.end_sentence();
    separated_effects(g, a, before, after);
}
static LEADING_BOUNDARY: CardDef = CardDef {
    name: "Replacement beginning with an explicit boundary",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    replacements: &[replacement(
        ReplacementKind::Damage,
        printed_damage,
        printed_unchanged,
        "Separate instructions.",
    )
    .then(leading_boundary)],
    ..CardDef::BLANK
};
static UNSAID: CardDef = CardDef {
    name: "Replacement with unsaid instructions",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    replacements: &[replacement(
        ReplacementKind::Damage,
        printed_damage,
        printed_unchanged,
        "Two instructions.",
    )
    .then(two_effects)],
    ..CardDef::BLANK
};
static GROUPED: CardDef = CardDef {
    name: "Replacement with one grouped instruction",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    replacements: &[replacement(
        ReplacementKind::Damage,
        printed_damage,
        printed_unchanged,
        "One instruction.",
    )
    .then(grouped_effects)],
    ..CardDef::BLANK
};
static SEPARATED: CardDef = CardDef {
    name: "Replacement with separated instructions",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    replacements: &[replacement(
        ReplacementKind::Damage,
        printed_damage,
        printed_unchanged,
        "Separate instructions.",
    )
    .then(separated_effects)],
    ..CardDef::BLANK
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &BEAR,
        &GRANTER,
        &UNSAID,
        &GROUPED,
        &SEPARATED,
        &LEADING_BOUNDARY,
    ])));
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    );
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
    });
    g.verify = true;
    g
}
fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: 0,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![], damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn resolving(cx: &mut Cx, source: ObjectId) {
    cx.g.check = StateCheck {
        depth: 1,
        resolving: Some(source),
        ..StateCheck::NONE
    };
}
fn add_replacement(cx: &mut Cx, source: ObjectId, target: ObjectId, index: u8) {
    let card = cx.obj(source).card;
    cx.add_effect(
        source,
        target,
        EffectKind::Replacement { card, index },
        Duration::EndOfTurn,
    );
}
#[test]
fn consuming_a_once_only_replacement_and_healing_is_one_core_action() {
    let mut g = game();
    let source = put(&mut g, &GRANTER, YOU, Zone::graveyard(YOU));
    let target = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    g.state.update_obj(target, |o| {
        o.damage = 1;
        o.deathtouched = true;
    });
    let mut cx = cx(&mut g);
    add_replacement(&mut cx, source, target, 0);
    resolving(&mut cx, source);
    assert!(cx.destroy_all(&[target]).is_empty());
    assert!(cx.obj(target).zone.is(ZoneKind::Battlefield));
    assert_eq!(cx.obj(target).damage, 0);
    assert!(!cx.obj(target).deathtouched);
    assert!(cx.state().effects.is_empty());
    cx.end_sentence();
    assert_eq!(cx.destroy_all(&[target]).len(), 1);
}
#[test]
fn exhausting_a_partial_shield_before_a_once_only_replacement_is_not_a_card_instruction() {
    let mut g = game();
    let source = put(&mut g, &GRANTER, YOU, Zone::graveyard(YOU));
    let attacker = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    let card = cx.obj(source).card;
    cx.add_effect(
        source,
        source,
        EffectKind::Prevention {
            card,
            index: 0,
            to: Target::Player(YOU),
            shield: Shield::Next(1),
        },
        Duration::EndOfTurn,
    );
    add_replacement(&mut cx, source, attacker, 1);
    cx.st()
        .update_frame(0, |f| f.answers.push((None, Answer::Index(0))));
    resolving(&mut cx, attacker);
    cx.doer = Some(THEM);
    assert_eq!(cx.damage(attacker, Target::Player(YOU), 3), 2);
    assert_eq!(cx.state().player(YOU).life, 18);
    assert_eq!(cx.state().player(THEM).life, 18);
    assert!(cx.state().effects.is_empty());
    assert_eq!(cx.doer, Some(THEM));
    assert!(cx.lineage.is_empty());
    assert_eq!(cx.g.check.depth, 1);
    assert_eq!(cx.g.check.resolving, Some(attacker));
    assert!(!cx.g.check.due);
    assert!(!cx.g.check.open);
    assert!(!cx.g.check.done);
}
#[test]
fn several_once_only_replacements_of_one_event_are_one_core_action() {
    let mut g = game();
    let source = put(&mut g, &GRANTER, YOU, Zone::graveyard(YOU));
    let attacker = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    add_replacement(&mut cx, source, attacker, 2);
    add_replacement(&mut cx, source, attacker, 2);
    cx.st()
        .update_frame(0, |f| f.answers.push((None, Answer::Index(0))));
    resolving(&mut cx, attacker);
    assert_eq!(cx.damage(attacker, Target::Player(YOU), 3), 3);
    assert_eq!(cx.state().player(YOU).life, 17);
    assert!(cx.state().effects.is_empty());
}
#[test]
fn simultaneous_saved_destructions_share_a_core_instruction() {
    let mut g = game();
    let source = put(&mut g, &GRANTER, YOU, Zone::graveyard(YOU));
    let one = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let two = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    for target in [one, two] {
        g.state.update_obj(target, |o| o.damage = 1);
    }
    let mut cx = cx(&mut g);
    add_replacement(&mut cx, source, one, 0);
    add_replacement(&mut cx, source, two, 0);
    resolving(&mut cx, source);
    assert!(cx.destroy_all(&[one, two]).is_empty());
    assert_eq!(cx.obj(one).damage, 0);
    assert_eq!(cx.obj(two).damage, 0);
    assert!(cx.state().effects.is_empty());
}
#[test]
fn simultaneous_counter_placements_share_their_replacement_instruction() {
    let mut g = game();
    let source = put(&mut g, &GRANTER, YOU, Zone::graveyard(YOU));
    let one = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let two = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    add_replacement(&mut cx, source, one, 3);
    add_replacement(&mut cx, source, two, 3);
    resolving(&mut cx, source);
    cx.doer = Some(YOU);
    cx.add_counters_together(&[
        (one, CounterKind::PlusOne, 1),
        (two, CounterKind::PlusOne, 1),
    ]);
    assert_eq!(cx.obj(one).counters(CounterKind::PlusOne), 2);
    assert_eq!(cx.obj(two).counters(CounterKind::PlusOne), 2);
    assert!(cx.state().effects.is_empty());
    assert_eq!(
        cx.g.log
            .iter()
            .filter(|r| matches!(r.event, Event::CountersPut { .. }))
            .count(),
        2
    );
}
#[test]
fn replacement_callbacks_must_still_state_their_own_instruction_boundaries() {
    let mut g = game();
    let _source = put(&mut g, &UNSAID, YOU, Zone::BATTLEFIELD);
    let attacker = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    resolving(&mut cx, attacker);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cx.damage(
            attacker,
            Target::Player(YOU),
            1
        )))
        .is_err(),
        "a core scope must not conceal two unannounced callback instructions"
    );
}
#[test]
fn replacement_callbacks_can_group_or_separate_their_instructions() {
    for def in [&GROUPED, &SEPARATED] {
        let mut g = game();
        let source = put(&mut g, def, YOU, Zone::BATTLEFIELD);
        let attacker = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
        let mut cx = cx(&mut g);
        resolving(&mut cx, attacker);
        assert_eq!(cx.damage(attacker, Target::Player(YOU), 1), 1);
        assert!(cx.chars(source).has(Keyword::Flying));
        assert!(cx.chars(source).has(Keyword::Haste));
    }
}
#[test]
fn an_ordinary_card_cannot_silently_perform_two_instructions() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    resolving(&mut cx, source);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cx.add_effect(
                source,
                source,
                EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
                Duration::EndOfTurn,
            );
            cx.add_effect(
                source,
                source,
                EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])),
                Duration::EndOfTurn,
            );
        }))
        .is_err(),
        "ordinary missing-boundary verification remains active"
    );
}

#[test]
fn a_replacement_callback_cannot_erase_an_unannounced_caller_instruction() {
    let mut g = game();
    let _source = put(&mut g, &GROUPED, YOU, Zone::BATTLEFIELD);
    let attacker = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    resolving(&mut cx, attacker);
    cx.add_effect(
        attacker,
        attacker,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
        Duration::EndOfTurn,
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cx.damage(
            attacker,
            Target::Player(YOU),
            1
        )))
        .is_err(),
        "entering a callback must not forgive the caller's pending instruction"
    );
}

#[test]
fn a_callback_boundary_is_propagated_even_when_it_changes_the_game_again() {
    let mut g = game();
    let first = put(&mut g, &LEADING_BOUNDARY, YOU, Zone::BATTLEFIELD);
    let second = put(&mut g, &GROUPED, YOU, Zone::BATTLEFIELD);
    let attacker = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let mut cx = cx(&mut g);
    resolving(&mut cx, attacker);
    cx.add_effect(
        attacker,
        attacker,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
        Duration::EndOfTurn,
    );
    cx.st()
        .update_frame(0, |f| f.answers.push((None, Answer::Index(0))));
    assert_eq!(cx.damage(attacker, Target::Player(YOU), 1), 1);
    for source in [first, second] {
        assert!(cx.chars(source).has(Keyword::Flying));
        assert!(cx.chars(source).has(Keyword::Haste));
    }
    assert_eq!(cx.g.check.depth, 1);
    assert_eq!(cx.g.check.resolving, Some(attacker));
    assert!(!cx.g.check.due);
}
