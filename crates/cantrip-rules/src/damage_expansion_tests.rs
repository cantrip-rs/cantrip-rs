//! Damage introduced by a replacement belongs to the same event (CR 120.4b-d, 614.6).
use crate::{Cx, Game, Setup, StateCheck};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static SOURCE: CardDef = CardDef {
    name: "Damage expansion source",
    types: TypeSet::of(&[CardType::Instant]),
    ..CardDef::BLANK
};
fn unlife(q: &dyn Query, a: &Asker, player: PlayerId) -> bool {
    player == a.controller && q.state().player(player).life <= 0
}
static UNLIFE: CardDef = CardDef {
    name: "Damage expansion Unlife",
    types: TypeSet::of(&[CardType::Enchantment]),
    infect_damage_to: &[unlife],
    loss_rules: &[cantrip_mechanics::statics::you_cant_lose_from_life],
    ..CardDef::BLANK
};
fn damage_to_you(_: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Player(a.controller) && d.amount > 0)
}
fn unchanged(_: &dyn Query, _: &Asker, _: &mut Proposal) {}
fn zero(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = 0;
    }
}
fn old_extra(g: &mut dyn Rules, a: &Asker, before: &Proposal, _: &Proposal) {
    if let Proposal::Damage(d) = before {
        g.damage_together(&[(a.source, Target::Player(g.controller(d.source)), d.amount)]);
    }
}
fn old_reformed(g: &mut dyn Rules, a: &Asker, before: &Proposal, _: &Proposal) {
    if let Proposal::Damage(d) = before {
        g.damage_together(&[
            (d.source, d.target, d.amount),
            (a.source, Target::Player(g.controller(d.source)), d.amount),
        ]);
    }
}
static OLD_EXTRA: CardDef = CardDef {
    name: "Damage expansion nested extra",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        unchanged,
        "also deals damage",
    )
    .then(old_extra)],
    ..UNLIFE
};
static OLD_REFORMED: CardDef = CardDef {
    name: "Damage expansion nested reformed",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        zero,
        "instead deals both",
    )
    .then(old_reformed)],
    ..UNLIFE
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &SOURCE,
        &UNLIFE,
        &OLD_EXTRA,
        &OLD_REFORMED,
        &EXTRA,
        &SPLIT,
        &INFECT_EXTRA,
        &BEAR,
        &DOUBLER,
        &GRANT,
        &SHIELD,
        &DEFLECTION,
        &INFECT_SOURCE,
        &BLAST,
        &INVALID,
        &INVALID_GRANT,
        &TOXIC_SOURCE,
        &COMBAT_SHIELD,
        &SPLIT_GRANT,
        &REDIRECT_CREATURE,
        &URZA_ARMOR,
        &PIT_SOURCE,
        &PREVENT_ALL,
        &QUANTITY_PROBE,
        &FILTERED_PROBE,
        &THRESHOLD_PROBE,
        &ENCHANTED_DAMAGE_PROBE,
        &SHARED_DAMAGE_PROBE,
        &CANT_POISON,
        &SAME_SOURCE_ADDITION,
        &SAME_TARGET_REDIRECT,
        &LEGACY_GAIN,
        &SCOPED_DOUBLER,
        &SCOPED_HALVER,
        &FRACTION_PREVENTION,
        &PRESENCE,
        &GHOSTS,
        &ARMOR_THREE,
        &NESTED_SAME_SOURCE,
        &PURE_SAME_SOURCE,
        &STATE_CAP,
        &STATE_CAP_ONLY,
        &STATE_CAP_ZERO,
        &REDIRECT_OTHER_TO_YOU,
        &PLAIN_DOUBLER,
    ])));
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![vec![cards.id_of(&SOURCE); 20]; 2],
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
fn put(g: &mut Game, card: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g
        .state
        .push_obj(Obj::new(g.cards.id_of(card), controller, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}
fn cx(g: &mut Game, source: ObjectId) -> Cx<'_> {
    g.check = StateCheck {
        depth: 1,
        resolving: Some(source),
        ..StateCheck::NONE
    };
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
#[test]
fn ordinary_nested_extra_is_a_separate_damage_event() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &OLD_EXTRA, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut cx = cx(&mut g, source);
    cx.damage(source, Target::Player(YOU), 2);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(YOU).poison()),
        (-1, 2)
    );
}
#[test]
fn existing_api_can_reform_a_single_hit_without_false_prevention() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &OLD_REFORMED, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut cx = cx(&mut g, source);
    cx.damage(source, Target::Player(YOU), 2);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(YOU).poison()),
        (-3, 0)
    );
    assert!(
        !cx.g
            .log
            .iter()
            .any(|r| matches!(r.event, Event::DamagePrevented { .. }))
    );
    assert_eq!(
        cx.g.log
            .iter()
            .filter(|r| matches!(r.event, Event::Damage { .. }))
            .count(),
        2
    );
}
#[test]
fn a_nested_reformed_batch_is_still_separate_from_original_siblings() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &OLD_REFORMED, THEM);
    put(&mut g, &UNLIFE, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut cx = cx(&mut g, source);
    cx.damage_together(&[
        (source, Target::Player(THEM), 2),
        (source, Target::Player(YOU), 1),
    ]);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(YOU).poison()),
        (-1, 1)
    );
}

fn extra(q: &dyn Query, a: &Asker, before: &DamageEvent, _: &DamageEvent) -> Vec<DamageComponent> {
    vec![DamageComponent::Additional(DamageEvent {
        source: a.source,
        target: Target::Player(q.controller(before.source)),
        amount: before.amount,
        combat: false,
        heal_marked: false,
        shielded: None,
    })]
}
static EXTRA: CardDef = CardDef {
    name: "Damage expansion simultaneous",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        unchanged,
        "also deals damage",
    )
    .with_damage(extra)],
    ..UNLIFE
};
fn split(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = d.amount.saturating_sub(2);
    }
}
fn redirected(
    _: &dyn Query,
    _: &Asker,
    before: &DamageEvent,
    after: &DamageEvent,
) -> Vec<DamageComponent> {
    vec![DamageComponent::Redirected {
        target: Target::Player(THEM),
        amount: before.amount - after.amount,
    }]
}
static SPLIT: CardDef = CardDef {
    name: "Damage expansion Harm's Way",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        split,
        "redirect two",
    )
    .with_damage(redirected)],
    ..UNLIFE
};
static BEAR: CardDef = CardDef {
    name: "Damage expansion creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    ..CardDef::BLANK
};
fn to_you(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Player(YOU) && d.amount > 0)
}
fn infect_creature(
    q: &dyn Query,
    a: &Asker,
    before: &DamageEvent,
    _: &DamageEvent,
) -> Vec<DamageComponent> {
    let target = q
        .state()
        .battlefield()
        .iter()
        .copied()
        .find(|&o| q.chars(o).is(CardType::Creature))
        .unwrap();
    vec![DamageComponent::Additional(DamageEvent {
        source: a.source,
        target: Target::Object(target),
        amount: before.amount,
        combat: false,
        heal_marked: false,
        shielded: None,
    })]
}
static INFECT_EXTRA: CardDef = CardDef {
    name: "Damage expansion infect source",
    replacements: &[replacement(
        ReplacementKind::Damage,
        to_you,
        unchanged,
        "also deals infect damage",
    )
    .with_damage(infect_creature)],
    keywords: Keywords::of(&[Keyword::Infect]),
    ..UNLIFE
};
#[test]
fn expanded_damage_has_no_results_until_all_original_and_added_parts_are_formed() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &EXTRA, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut cx = cx(&mut g, source);
    assert_eq!(cx.damage(source, Target::Player(YOU), 2), 2);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(YOU).poison()),
        (-3, 0)
    );
    assert!(
        !cx.g
            .log
            .iter()
            .any(|r| matches!(r.event, Event::DamagePrevented { .. }))
    );
}
#[test]
fn expanded_damage_stays_simultaneous_with_original_sibling_assignments() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &EXTRA, THEM);
    put(&mut g, &UNLIFE, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut cx = cx(&mut g, source);
    assert_eq!(
        cx.damage_together(&[
            (source, Target::Player(THEM), 2),
            (source, Target::Player(YOU), 1)
        ]),
        [2, 1]
    );
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(YOU).poison()),
        (-2, 0)
    );
}
#[test]
fn partial_redirection_forms_damage_instead_of_preventing_it() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, THEM);
    put(&mut g, &SPLIT, YOU);
    let mut cx = cx(&mut g, source);
    let dealt = cx.damage(source, Target::Player(YOU), 5);
    assert_eq!(
        dealt, 5,
        "all original damage fragments count; no additional source does"
    );
    cx.gain_life(THEM, dealt.min(i32::MAX as u32) as i32);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(THEM).life),
        (17, 23)
    );
    assert!(
        !cx.g
            .log
            .iter()
            .any(|r| matches!(r.event, Event::DamagePrevented { .. }))
    );
    assert_eq!(
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s, amount, ..
                } if s == source => Some(amount),
                _ => None,
            })
            .sum::<u32>(),
        5
    );
}
#[test]
fn added_sources_keep_their_controller_and_counter_history() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let extra = put(&mut g, &INFECT_EXTRA, THEM);
    let bear = put(&mut g, &BEAR, YOU);
    let mut cx = cx(&mut g, source);
    cx.damage(source, Target::Player(YOU), 2);
    assert_eq!(cx.obj(bear).counters(CounterKind::MinusOne), 2);
    assert!(cx.g.log.iter().any(
        |r| matches!(r.event,Event::Damage {source:s,controller:THEM,amount:2,..} if s==extra)
    ));
    assert!(cx.g.log.iter().any(|r| matches!(r.event,Event::CountersPut {object,by:THEM,kind:CounterKind::MinusOne,amount:2,..} if object==bear)));
}

fn any_damage(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.amount > 0)
}
fn double(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount *= 2;
    }
}
static DOUBLER: CardDef = CardDef {
    name: "Damage expansion doubler",
    replacements: &[replacement(
        ReplacementKind::Damage,
        any_damage,
        double,
        "double damage",
    )],
    ..UNLIFE
};
fn granted_applies(_: &dyn Query, a: &Asker, affected: ObjectId, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.source == affected && d.target == Target::Player(a.controller) && d.amount > 0)
}
fn granted_noop(_: &dyn Query, _: &Asker, _: ObjectId, _: &mut Proposal) {}
fn granted_extra(
    q: &dyn Query,
    a: &Asker,
    _: ObjectId,
    b: &DamageEvent,
    d: &DamageEvent,
) -> Vec<DamageComponent> {
    extra(q, a, b, d)
}
static GRANT: CardDef = CardDef {
    name: "Damage expansion once-only Eye",
    granted_replacements: &[granted_replacement(
        ReplacementKind::Damage,
        granted_applies,
        granted_noop,
        "also deals damage once",
    )
    .once()
    .with_damage(granted_extra)],
    ..SOURCE
};
static SHIELD: CardDef = CardDef {
    name: "Damage expansion prevention",
    granted_preventions: &[cantrip_mechanics::prevention::ANY_DAMAGE],
    ..SOURCE
};
fn deflect(g: &mut dyn Rules, a: &Asker, _: ObjectId, parts: &[Prevented]) {
    g.damage(
        a.source,
        Target::Player(YOU),
        Prevented::total(parts).try_into().unwrap(),
    );
}
static DEFLECTION: CardDef = CardDef {
    name: "Damage expansion Divine Deflection",
    granted_preventions: &[GrantedPreventionDef {
        then: Some(deflect),
        ..cantrip_mechanics::prevention::ANY_DAMAGE
    }],
    ..SOURCE
};
static INFECT_SOURCE: CardDef = CardDef {
    name: "Damage expansion combat infect",
    types: TypeSet::of(&[CardType::Creature]),
    keywords: Keywords::of(&[Keyword::Infect]),
    toxic: 1,
    power: Some(5),
    toughness: Some(5),
    ..SOURCE
};
fn answers(cx: &mut Cx, answers: Vec<Answer>) {
    cx.st().update_frame(0, |f| {
        f.answers.extend(answers.into_iter().map(|a| (None, a)))
    });
}
fn shield(cx: &mut Cx, source: ObjectId, to: Target, amount: u32) {
    cx.add_effect(
        source,
        source,
        EffectKind::Prevention {
            card: cx.obj(source).card,
            index: 0,
            to,
            shield: Shield::Next(amount),
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
}
#[test]
fn earlier_replacements_are_not_reapplied_to_added_components() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, THEM);
    put(&mut g, &DOUBLER, YOU);
    let eye = put(&mut g, &EXTRA, YOU);
    let mut cx = cx(&mut g, source);
    answers(&mut cx, vec![Answer::Index(0)]);
    assert_eq!(cx.damage(source, Target::Player(YOU), 2), 4);
    let damage: Vec<_> =
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage { source, amount, .. } => Some((source, amount)),
                _ => None,
            })
            .collect();
    assert_eq!(damage, [(source, 4), (eye, 4)]);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(THEM).life),
        (16, 16)
    );
    assert!(cx.lineage.is_empty());
    assert_eq!(cx.g.check.resolving, Some(source));
    assert_eq!(cx.g.check.depth, 1);
}
#[test]
fn chained_expansions_terminate_with_each_branchs_applied_lineage() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &EXTRA, YOU);
    put(&mut g, &EXTRA, YOU);
    let mut cx = cx(&mut g, source);
    answers(&mut cx, vec![Answer::Index(0)]);
    assert_eq!(cx.damage(source, Target::Player(YOU), 2), 2);
    assert_eq!(cx.state().player(YOU).life, 12);
    assert_eq!(
        cx.g.log
            .iter()
            .filter(|r| matches!(r.event, Event::Damage { .. }))
            .count(),
        3
    );
    let mut amounts: Vec<_> = cx.g.log.iter().filter_map(|r| match r.event { Event::Damage { amount, .. } => Some(amount), _ => None }).collect();
    amounts.sort_unstable();
    assert_eq!(amounts, [2, 2, 4]);
    assert!(cx.lineage.is_empty());
}
#[test]
fn a_new_source_can_receive_the_remaining_prevention_shield_first() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let eye = put(&mut g, &EXTRA, YOU);
    let protection = put(&mut g, &SHIELD, YOU);
    let mut cx = cx(&mut g, source);
    shield(&mut cx, protection, Target::Player(YOU), 2);
    answers(&mut cx, vec![Answer::Index(0), Answer::Cards(vec![eye])]);
    assert_eq!(cx.damage(source, Target::Player(YOU), 2), 2);
    assert_eq!(cx.state().player(YOU).life, 18);
    assert!(
        cx.g.log
            .iter()
            .any(|r| matches!(r.event,Event::DamagePrevented {source:s,amount:2,..} if s==eye))
    );
    assert!(cx.state().effects.is_empty());
}
#[test]
fn partial_prevention_then_a_once_only_expansion_keeps_the_remaining_amount() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, THEM);
    let eye = put(&mut g, &GRANT, YOU);
    let protection = put(&mut g, &SHIELD, YOU);
    let mut cx = cx(&mut g, source);
    shield(&mut cx, protection, Target::Player(YOU), 1);
    cx.add_effect(
        eye,
        source,
        EffectKind::Replacement {
            card: cx.obj(eye).card,
            index: 0,
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
    answers(&mut cx, vec![Answer::Index(1)]);
    assert_eq!(cx.damage(source, Target::Player(YOU), 3), 2);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(THEM).life),
        (18, 18)
    );
    assert!(cx.state().effects.is_empty());
    assert_eq!(cx.damage(source, Target::Player(YOU), 3), 3);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(THEM).life),
        (15, 18)
    );
}
#[test]
fn prevention_followup_damage_is_after_the_original_event_not_simultaneous() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, THEM);
    let deflection = put(&mut g, &DEFLECTION, YOU);
    put(&mut g, &UNLIFE, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut cx = cx(&mut g, source);
    shield(&mut cx, deflection, Target::Player(YOU), 2);
    assert_eq!(cx.damage(source, Target::Player(YOU), 3), 1);
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(YOU).poison()),
        (0, 2)
    );
    let damage: Vec<_> =
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage { source, amount, .. } => Some((source, amount)),
                _ => None,
            })
            .collect();
    assert_eq!(damage, [(source, 1), (deflection, 2)]);
}
#[test]
fn original_redirected_fragments_retain_combat_infect_and_toxic() {
    let mut g = game();
    let source = put(&mut g, &INFECT_SOURCE, THEM);
    put(&mut g, &SPLIT, YOU);
    let mut cx = cx(&mut g, source);
    let amounts = cx.instruction(
        |cx| {
            let (outcomes, _, mut batch) =
                crate::ops::damage_batch(cx, &[(source, Target::Player(YOU), 5)], true);
            batch.rest_of_preventions(cx);
            outcomes.into_iter().map(|o| o.amount).collect::<Vec<_>>()
        },
        |_| true,
    );
    assert_eq!(amounts, [5]);
    assert_eq!(
        (
            cx.state().player(YOU).poison(),
            cx.state().player(THEM).poison()
        ),
        (4, 3)
    );
    assert!(
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage { kind, .. } => Some(kind.is_combat()),
                _ => None,
            })
            .all(|combat| combat)
    );
}
fn blast(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, Target::Player(YOU), 2);
    Ok(())
}
static BLAST: CardDef = CardDef {
    name: "Damage expansion cast spell",
    spell: spell(&[], blast),
    ..SOURCE
};
fn pending(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("pending native question")
}
fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("legal native answer")
}
#[test]
fn native_cast_resolution_and_goto_restore_expanded_damage_and_history() {
    let mut g = game();
    put(&mut g, &EXTRA, YOU);
    let probe = put(&mut g, &QUANTITY_PROBE, YOU);
    g.state.update_player(YOU, |p| p.life = 1);
    g.state.frame_remove(0);
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let source = g
        .state
        .push_obj(Obj::new(g.cards.id_of(&BLAST), YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), source);
    let r = pending(&mut g);
    let RequestKind::Priority { options } = &r.kind else {
        panic!("priority")
    };
    let action = options
        .iter()
        .find(|o| o.source == Some(source) && o.kind == ActionKind::CastSpell)
        .expect("actual cast offered")
        .clone();
    answer(&mut g, &r, Answer::Act(Action::of(&action)));
    loop {
        let r = pending(&mut g);
        match &r.kind {
            RequestKind::PayCosts { options, .. } => {
                let confirm = options
                    .iter()
                    .find(|o| o.kind == ActionKind::ConfirmCasting)
                    .unwrap()
                    .clone();
                answer(&mut g, &r, Answer::Act(Action::of(&confirm)));
            }
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected cast question {other:?}"),
        }
    }
    let cast = g.at;
    assert_eq!(g.state.stack.len(), 1);
    let mut triggered_at = None;
    loop {
        let r = pending(&mut g);
        if g.state.stack.is_empty() && g.state.triggers.is_empty() { break; }
        if let Some(item) = g.state.stack.iter().find(|i| i.object == probe) {
            assert_eq!(item.matched_damage_amount(), 4);
            triggered_at = Some(g.at);
        }
        assert!(matches!(r.kind, RequestKind::Priority { .. }));
        answer(&mut g, &r, Answer::Pass);
    }
    g.run();
    let resolved = g.at;
    assert_eq!(g.state.player(THEM).life, 24);
    assert_eq!(
        (g.state.player(YOU).life, g.state.player(YOU).poison()),
        (-3, 0)
    );
    assert_eq!(
        g.log
            .iter()
            .filter(|r| matches!(r.event, Event::Damage { .. }))
            .count(),
        2
    );
    g.goto(cast);
    assert_eq!(
        (g.state.player(YOU).life, g.state.player(YOU).poison()),
        (1, 0)
    );
    assert!(
        !g.log
            .iter()
            .any(|r| matches!(r.event, Event::Damage { .. }))
    );
    g.goto(triggered_at.expect("frozen quantity was put on the native stack"));
    assert_eq!(g.state.stack.iter().find(|i| i.object == probe).unwrap().matched_damage_amount(), 4);
    g.goto(resolved);
    assert_eq!(g.state.player(THEM).life, 24);
    assert_eq!(
        (g.state.player(YOU).life, g.state.player(YOU).poison()),
        (-3, 0)
    );
    assert_eq!(
        g.log
            .iter()
            .filter(|r| matches!(r.event, Event::Damage { .. }))
            .count(),
        2
    );
}

fn invalid(
    _: &dyn Query,
    a: &Asker,
    before: &DamageEvent,
    after: &DamageEvent,
) -> Vec<DamageComponent> {
    vec![DamageComponent::Redirected {
        target: Target::Object(a.source),
        amount: before.amount - after.amount,
    }]
}
static INVALID: CardDef = CardDef {
    name: "Damage expansion invalid redirect",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        split,
        "partial redirection to noncreature",
    )
    .with_damage(invalid)],
    ..UNLIFE
};
fn granted_split(_: &dyn Query, _: &Asker, _: ObjectId, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = d.amount.saturating_sub(2);
    }
}

fn granted_invalid(
    q: &dyn Query,
    a: &Asker,
    _: ObjectId,
    b: &DamageEvent,
    d: &DamageEvent,
) -> Vec<DamageComponent> {
    invalid(q, a, b, d)
}
static INVALID_GRANT: CardDef = CardDef {
    name: "Damage expansion invalid once redirect",
    granted_replacements: &[granted_replacement(
        ReplacementKind::Damage,
        granted_applies,
        granted_split,
        "once-only partial redirection to noncreature",
    )
    .once()
    .with_damage(granted_invalid)],
    ..SOURCE
};
#[test]
fn impossible_partial_redirection_leaves_the_original_damage_intact() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, THEM);
    put(&mut g, &INVALID, YOU);
    let mut cx = cx(&mut g, source);
    assert_eq!(cx.damage(source, Target::Player(YOU), 5), 5);
    assert_eq!(cx.state().player(YOU).life, 15);
}
#[test]
fn impossible_partial_redirection_does_not_consume_its_once_only_effect() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, THEM);
    let redirect = put(&mut g, &INVALID_GRANT, YOU);
    let mut cx = cx(&mut g, source);
    cx.add_effect(
        redirect,
        source,
        EffectKind::Replacement {
            card: cx.obj(redirect).card,
            index: 0,
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
    assert_eq!(cx.damage(source, Target::Player(YOU), 5), 5);
    assert_eq!(cx.state().player(YOU).life, 15);
    assert_eq!(cx.state().effects.len(), 1);
}

static TOXIC_SOURCE: CardDef = CardDef {
    name: "Damage expansion toxic attacker",
    toxic: 2,
    ..BEAR
};
fn only_combat(_: &dyn Query, _: &Asker, _: ObjectId, d: &DamageEvent) -> bool {
    d.combat
}
static COMBAT_SHIELD: CardDef = CardDef {
    name: "Damage expansion combat prevention",
    granted_preventions: &[GrantedPreventionDef {
        applies: only_combat,
        ..cantrip_mechanics::prevention::ANY_DAMAGE
    }],
    ..SOURCE
};
#[test]
fn eye_addition_is_noncombat_even_when_original_creature_damage_is_combat_and_toxic() {
    // CR120.2a-b: combat is the creature's assignment, while Eye is a new
    // spell source (cached Eye ruling2009-02-01). The sources share an event,
    // not a damage classification. A combat-only shield distinguishes them.
    let mut g = game();
    let source = put(&mut g, &TOXIC_SOURCE, THEM);
    let eye = put(&mut g, &EXTRA, YOU);
    let protection = put(&mut g, &COMBAT_SHIELD, THEM);
    let mut cx = cx(&mut g, source);
    cx.add_effect(
        protection,
        protection,
        EffectKind::Prevention {
            card: cx.obj(protection).card,
            index: 0,
            to: Target::Player(THEM),
            shield: Shield::All,
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
    cx.instruction(
        |cx| {
            let (_, _, mut batch) =
                crate::ops::damage_batch(cx, &[(source, Target::Player(YOU), 2)], true);
            batch.rest_of_preventions(cx);
        },
        |_| true,
    );
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(THEM).life),
        (18, 18)
    );
    assert_eq!(
        (
            cx.state().player(YOU).poison(),
            cx.state().player(THEM).poison()
        ),
        (2, 0)
    );
    let kinds: Vec<_> =
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage { source, kind, .. } => Some((source, kind.is_combat())),
                _ => None,
            })
            .collect();
    assert_eq!(kinds, [(source, true), (eye, false)]);
    assert!(
        !cx.g
            .log
            .iter()
            .any(|r| matches!(r.event, Event::DamagePrevented { .. }))
    );
}

fn granted_redirected(
    q: &dyn Query,
    a: &Asker,
    _: ObjectId,
    b: &DamageEvent,
    d: &DamageEvent,
) -> Vec<DamageComponent> {
    redirected(q, a, b, d)
}
static SPLIT_GRANT: CardDef = CardDef {
    name: "Damage expansion chosen-source partial redirect",
    granted_replacements: &[granted_replacement(
        ReplacementKind::Damage,
        granted_applies,
        granted_split,
        "redirect part of chosen source damage",
    )
    .with_damage(granted_redirected)],
    ..SOURCE
};
#[test]
fn once_only_eye_replaces_every_eligible_component_after_partial_redirection() {
    // One source deals1toYOU and2toTHEM. Harm's Way redirects the first
    // component toTHEM before their Eye applies. The chosen source now deals
    // 3toTHEM in this one event: Eye reflects all3, not just the first part.
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let redirect = put(&mut g, &SPLIT_GRANT, YOU);
    let eye = put(&mut g, &GRANT, THEM);
    let mut cx = cx(&mut g, source);
    cx.add_effect(
        redirect,
        source,
        EffectKind::Replacement {
            card: cx.obj(redirect).card,
            index: 0,
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
    cx.add_effect(
        eye,
        source,
        EffectKind::Replacement {
            card: cx.obj(eye).card,
            index: 0,
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
    assert_eq!(
        cx.damage_together(&[
            (source, Target::Player(YOU), 1),
            (source, Target::Player(THEM), 2)
        ]),
        [1, 2]
    );
    assert_eq!(
        (cx.state().player(YOU).life, cx.state().player(THEM).life),
        (17, 17)
    );
    let reflected: u32 =
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s, amount, ..
                } if s == eye => Some(amount),
                _ => None,
            })
            .sum();
    assert_eq!(reflected, 3);
    assert_eq!(
        cx.state()
            .effects
            .iter()
            .filter(|e| e.source == eye)
            .count(),
        0
    );
}

fn creature_source(q: &dyn Query, a: &Asker, affected: ObjectId, p: &Proposal) -> bool {
    matches!(p,Proposal::Damage(d) if d.source==affected && matches!(d.target,Target::Object(o) if q.obj(o).controller==a.controller) && d.amount>0)
}
fn redirect_all(_: &dyn Query, _: &Asker, _: ObjectId, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = 0;
    }
}
static REDIRECT_CREATURE: CardDef = CardDef {
    name: "Damage convergence Captain's Maneuver",
    granted_replacements: &[granted_replacement(
        ReplacementKind::Damage,
        creature_source,
        redirect_all,
        "redirect damage from the creature",
    )
    .with_damage(granted_redirected)],
    ..SOURCE
};
fn armor_to_you(_: &dyn Query, a: &Asker, d: &DamageEvent) -> bool {
    d.target == Target::Player(a.controller)
}
fn armor_one(_: &dyn Query, _: &Asker, _: &DamageEvent) -> u32 {
    1
}
static URZA_ARMOR: CardDef = CardDef {
    name: "Damage convergence Urza's Armor",
    preventions: &[PreventionDef {
        hint: "prevent1 from the source",
        applies: armor_to_you,
        amount: PreventionAmount::Specific(armor_one),
    }],
    ..UNLIFE
};
fn pit(_: &dyn Query, a: &Asker, e: &Event) -> Option<Fired> {
    match *e {
        Event::Damage {
            source,
            target: Target::Player(p),
            amount,
            ..
        } if source == a.source && amount > 0 => Fired::about(Target::Player(p)),
        _ => None,
    }
}
fn pit_poison(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.give_poison(p, 1);
    }
    Ok(())
}
static PIT_SOURCE: CardDef = CardDef {
    name: "Damage convergence Pit Scorpion and toxic",
    power: Some(3),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Trample]),
    toxic: 2,
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, pit),
        pit_poison,
        "Whenever this deals damage to a player, that player gets a poison counter.",
    )],
    ..BEAR
};
fn converging_position(armor: bool) -> (Game, ObjectId, ObjectId) {
    let mut g = game();
    let source = put(&mut g, &PIT_SOURCE, YOU);
    let bear = put(&mut g, &BEAR, THEM);
    let redirect = put(&mut g, &REDIRECT_CREATURE, THEM);
    if armor {
        put(&mut g, &URZA_ARMOR, THEM);
    }
    let mut cx = cx(&mut g, source);
    cx.add_effect(
        redirect,
        source,
        EffectKind::Replacement {
            card: cx.obj(redirect).card,
            index: 0,
        },
        Duration::EndOfTurn,
    );
    cx.end_sentence();
    drop(cx);
    (g, source, bear)
}
#[test]
fn converged_same_source_recipient_damage_is_one_record_toxic_and_nonbatch_trigger() {
    let (mut g, source, bear) = converging_position(false);
    let mut cx = cx(&mut g, source);
    cx.instruction(
        |cx| {
            let (r, _, mut b) = crate::ops::damage_batch(
                cx,
                &[
                    (source, Target::Object(bear), 1),
                    (source, Target::Player(THEM), 2),
                ],
                true,
            );
            assert_eq!(r.into_iter().map(|o| o.amount).collect::<Vec<_>>(), [1, 2]);
            b.rest_of_preventions(cx);
        },
        |_| true,
    );
    assert_eq!(
        (
            cx.state().player(THEM).life,
            cx.state().player(THEM).poison(),
            cx.g.log
                .iter()
                .filter(|r| matches!(r.event, Event::Damage { source: s, .. } if s == source))
                .count(),
            cx.state()
                .triggers
                .iter()
                .filter(|t| t.source == source)
                .count()
        ),
        (17, 2, 1, 1),
        "one source/recipient event means toxic once and one non-batch Pit Scorpion trigger"
    );
}
#[test]
fn converged_source_damage_meets_urzas_armor_only_once() {
    let (mut g, source, bear) = converging_position(true);
    let mut cx = cx(&mut g, source);
    cx.instruction(
        |cx| {
            let (r, _, mut b) = crate::ops::damage_batch(
                cx,
                &[
                    (source, Target::Object(bear), 1),
                    (source, Target::Player(THEM), 2),
                ],
                true,
            );
            assert_eq!(r.into_iter().map(|o| o.amount).sum::<u32>(), 2);
            b.rest_of_preventions(cx);
        },
        |_| true,
    );
    assert_eq!(cx.state().player(THEM).life, 18);
    assert_eq!(
        cx.g.log
            .iter()
            .filter(|r| matches!(r.event,Event::Damage {source:s,..} if s==source))
            .count(),
        1
    );
}
#[test]
fn converged_damage_beyond_u16_remains_one_exact_source_event() {
    // Earthquake-sized40000 to a creature and40000 to its controller;
    // Captain's Maneuver redirects the creature's portion to that controller.
    // All individual inputs are in the old API range. Armor prevents only1.
    let (mut g, source, bear) = converging_position(true);
    let mut cx = cx(&mut g, source);
    let (r, _, mut b) = crate::ops::damage_batch(
        &mut cx,
        &[
            (source, Target::Object(bear), 40_000),
            (source, Target::Player(THEM), 40_000),
        ],
        false,
    );
    assert_eq!(
        r.into_iter().map(|o| o.amount).sum::<u32>(),
        79_999
    );
    b.rest_of_preventions(&mut cx);
    assert_eq!(cx.state().player(THEM).life, 20 - 79_999);
    let damage: Vec<_> =
        cx.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s, amount, ..
                } if s == source => Some(amount),
                _ => None,
            })
            .collect();
    assert_eq!(damage, [79_999]);
}


static PREVENT_ALL: CardDef = CardDef {
    name: "Damage convergence unlimited prevention",
    preventions: &[PreventionDef { hint: "prevent all", applies: armor_to_you, amount: PreventionAmount::All }],
    ..UNLIFE
};

#[test]
fn finite_prevention_stays_used_when_later_redirected_damage_converges() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let redirect = put(&mut g, &SPLIT_GRANT, YOU);
    put(&mut g, &URZA_ARMOR, THEM);
    let mut cx = cx(&mut g, source);
    cx.add_effect(redirect, source, EffectKind::Replacement { card: cx.obj(redirect).card, index: 0 }, Duration::EndOfTurn);
    cx.end_sentence();
    assert_eq!(cx.damage_together(&[(source, Target::Player(THEM), 3), (source, Target::Player(YOU), 2)]), [2, 2]);
    assert_eq!(cx.state().player(THEM).life, 16);
    let amounts: Vec<_> = cx.g.log.iter().filter_map(|r| match r.event { Event::Damage { source: s, amount, .. } if s == source => Some(amount), _ => None }).collect();
    assert_eq!(amounts, [4]);
}

#[test]
fn prevention_at_original_source_does_not_suppress_prevention_of_eye_source() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &URZA_ARMOR, YOU);
    let eye = put(&mut g, &EXTRA, YOU);
    let mut cx = cx(&mut g, source);
    answers(&mut cx, vec![Answer::Index(0)]);
    assert_eq!(cx.damage(source, Target::Player(YOU), 3), 2);
    assert_eq!(cx.state().player(YOU).life, 17);
    let mut damage: Vec<_> = cx.g.log.iter().filter_map(|r| match r.event { Event::Damage { source, amount, .. } => Some((source, amount)), _ => None }).collect();
    damage.sort_by_key(|&(source, _)| source);
    assert_eq!(damage, [(source, 2), (eye, 1)]);
    let prevented: u32 = cx.g.log.iter().filter_map(|r| match r.event { Event::DamagePrevented { amount, .. } => Some(amount), _ => None }).sum();
    assert_eq!(prevented, 2);
}

#[test]
fn unlimited_prevention_also_prevents_new_arrivals_after_original_group_is_zero() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let redirect = put(&mut g, &SPLIT_GRANT, YOU);
    put(&mut g, &PREVENT_ALL, THEM);
    let mut cx = cx(&mut g, source);
    cx.add_effect(redirect, source, EffectKind::Replacement { card: cx.obj(redirect).card, index: 0 }, Duration::EndOfTurn);
    cx.end_sentence();
    assert_eq!(cx.damage_together(&[(source, Target::Player(THEM), 2), (source, Target::Player(YOU), 2)]), [0, 0]);
    assert_eq!(cx.state().player(THEM).life, 20);
    assert!(!cx.g.log.iter().any(|r| matches!(r.event, Event::Damage { .. })));
}

fn player_damage(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match *event { Event::Damage { target: Target::Player(p), .. } => Fired::about(Target::Player(p)), _ => None }
}
fn gain_quantity(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(THEM, item.matched_damage_amount().min(i32::MAX as u32) as i32);
    Ok(())
}
static QUANTITY_PROBE: CardDef = CardDef {
    name: "Damage exact frozen amount probe",
    abilities: &[triggered(when(EventMask::DAMAGE, player_damage), gain_quantity, "Whenever damage is dealt to a player, gain that much life.").damage_amount().sums_per_subject()],
    ..BEAR
};
fn filtered_damage(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::Damage { target: Target::Player(THEM), amount, kind, controller: YOU, .. }
            if amount > 0 && kind.is_combat() && kind.colors().has(Color::Red) => Fired::about(Target::Player(THEM)),
        _ => None,
    }
}
static FILTERED_PROBE: CardDef = CardDef {
    name: "Damage exact selected amount probe",
    abilities: &[triggered(when(EventMask::DAMAGE, filtered_damage), gain_quantity, "Whenever red combat damage controlled by you is dealt to an opponent, gain that much life.").damage_amount().sums_per_subject()],
    ..BEAR
};

#[test]
fn separate_damage_batches_keep_separate_frozen_quantities() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let watcher = put(&mut g, &QUANTITY_PROBE, YOU);
    let mut cx = cx(&mut g, source);
    cx.damage(source, Target::Player(THEM), 2);
    cx.damage(source, Target::Player(THEM), 3);
    let amounts: Vec<_> = cx.state().triggers.iter().filter(|t| t.source == watcher).map(|t| t.event_amount).collect();
    assert_eq!(amounts, [Some(2), Some(3)]);
}

#[test]
fn coalesced_original_assignments_keep_exact_returns_through_doubling() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &DOUBLER, YOU);
    let mut cx = cx(&mut g, source);
    assert_eq!(cx.damage_together(&[(source, Target::Player(THEM), 1), (source, Target::Player(THEM), 2)]), [2, 4]);
    assert_eq!(cx.state().player(THEM).life, 14);
    assert_eq!(cx.g.log.iter().filter(|r| matches!(r.event, Event::Damage { amount: 6, .. })).count(), 1);
}

#[test]
fn coalesced_original_assignments_keep_stable_returns_through_limited_prevention() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let prevention = put(&mut g, &SHIELD, THEM);
    let mut cx = cx(&mut g, source);
    shield(&mut cx, prevention, Target::Player(THEM), 1);
    assert_eq!(cx.damage_together(&[(source, Target::Player(THEM), 1), (source, Target::Player(THEM), 2)]), [0, 2]);
    assert_eq!(cx.state().player(THEM).life, 18);
    assert_eq!(cx.g.log.iter().filter(|r| matches!(r.event, Event::Damage { amount: 2, .. })).count(), 1);
}


fn no_poison(_: &dyn Query, a: &Asker, player: PlayerId, kind: PlayerCounter) -> bool {
    a.controller == player && kind == PlayerCounter::Poison
}
static CANT_POISON: CardDef = CardDef {
    name: "Damage width poison ban",
    player_counter_bans: &[no_poison],
    ..UNLIFE
};
fn add_same_source(_: &dyn Query, _: &Asker, before: &DamageEvent, _: &DamageEvent) -> Vec<DamageComponent> {
    vec![DamageComponent::Additional(DamageEvent { amount: 1, ..*before })]
}
static SAME_SOURCE_ADDITION: CardDef = CardDef {
    name: "Damage width additional same source",
    replacements: &[replacement(ReplacementKind::Damage, damage_to_you, unchanged, "also deals one").with_damage(add_same_source)],
    ..UNLIFE
};
#[test]
fn addition_can_share_the_final_damage_group_without_entering_original_return() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &SAME_SOURCE_ADDITION, YOU);
    let mut cx = cx(&mut g, source);
    assert_eq!(cx.damage(source, Target::Player(YOU), 2), 2);
    assert_eq!(cx.state().player(YOU).life, 17);
    assert_eq!(cx.g.log.iter().filter(|r| matches!(r.event, Event::Damage { amount: 3, .. })).count(), 1);
}

#[test]
fn native_ability_copy_and_state_replay_keep_the_frozen_damage_payload() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let probe = put(&mut g, &QUANTITY_PROBE, YOU);
    cx(&mut g, source).damage(source, Target::Player(THEM), 2);
    g.state.frame_remove(0);
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let request = pending(&mut g);
    assert!(matches!(request.kind, RequestKind::Priority { .. }));
    let original = *g.state.stack.iter().find(|i| i.object == probe).expect("matched quantity on stack");
    assert_eq!(original.matched_damage_amount(), 2);
    let before = g.state.clone();
    let mark = g.state.mark();
    {
        let mut cx = cx(&mut g, probe);
        let copied = cx.copy_ability(Target::Ability(original.serial), THEM, false).unwrap().expect("native ability copy");
        let item = cx.state().stack.iter().find(|i| i.serial == copied).unwrap();
        assert!(item.copied);
        assert_eq!(item.matched_damage_amount(), 2);
        assert_eq!(item.controller, THEM);
    }
    let (undos, redos) = g.state.step_back(mark);
    assert_eq!(g.state, before);
    g.state.step_forward(undos, redos);
    assert_eq!(g.state.stack.last().unwrap().matched_damage_amount(), 2);
    let mut missing = original;
    missing.data[0] = 99;
    missing.event_amount = None;
    assert_eq!(missing.matched_damage_amount(), 0, "a trigger that matched no damage carries none");
}

fn redirect_same_target(_: &dyn Query, _: &Asker, before: &DamageEvent, after: &DamageEvent) -> Vec<DamageComponent> {
    vec![DamageComponent::Redirected { target: before.target, amount: before.amount - after.amount }]
}
static SAME_TARGET_REDIRECT: CardDef = CardDef {
    name: "Damage same-target original fragments",
    replacements: &[replacement(ReplacementKind::Damage, damage_to_you, split, "redirect part to same recipient").with_damage(redirect_same_target)],
    ..UNLIFE
};
#[test]
fn recombined_original_fragments_form_one_precise_damage_result() {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    put(&mut g, &SAME_TARGET_REDIRECT, YOU);
    let mut cx = cx(&mut g, source);
    assert_eq!(cx.damage_result(source, Target::Player(YOU), 5), Some(DamageResult { target: Target::Player(YOU), amount: 5, excess: 0 }));
    assert_eq!(cx.state().player(YOU).life, 15);
    assert_eq!(cx.g.log.iter().filter(|r| matches!(r.event, Event::Damage { amount: 5, .. })).count(), 1);
}


fn opponent_loss(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { player: THEM, delta } if delta < 0 => Some(Fired { subject: Target::Player(THEM), data: [u32::try_from(delta.unsigned_abs()).unwrap(), 0] }),
        _ => None,
    }
}
static LEGACY_GAIN: CardDef = CardDef {
    name: "Damage width legacy actual life loss payload",
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, opponent_loss), cantrip_mechanics::effects::gain_that_much_life, "Whenever an opponent loses life, gain that much life.")],
    ..UNLIFE
};
// CR614.5 keeps the opportunity of an independently original assignment when redirection makes its recipient converge.
fn half(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount /= 2;
    }
}
static SCOPED_DOUBLER: CardDef = CardDef {
    name: "Damage lineage scoped doubler",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        double,
        "double damage to you",
    )],
    ..UNLIFE
};
static SCOPED_HALVER: CardDef = CardDef {
    name: "Damage lineage scoped halver",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        half,
        "half damage to you instead",
    )],
    ..UNLIFE
};
fn damage_to_other(_: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Player(THEM) && a.controller == YOU && d.amount > 0)
}
fn redirect_other(_: &dyn Query, a: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.target = Target::Player(a.controller);
    }
}
static REDIRECT_OTHER_TO_YOU: CardDef = CardDef {
    name: "Damage lineage direct redirect",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_other,
        redirect_other,
        "damage to that opponent is dealt to you instead",
    )],
    ..UNLIFE
};
fn scoped_converging(modifier: &'static CardDef) -> (Game, ObjectId) {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    put(&mut g, modifier, YOU);
    put(&mut g, &REDIRECT_OTHER_TO_YOU, YOU);
    (g, source)
}
#[test]
fn lineage_unmodified_original_converges_after_scoped_double() {
    let (mut g, source) = scoped_converging(&SCOPED_DOUBLER);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 2),
            (source, Target::Player(THEM), 3)
        ]),
        [4, 6]
    );
    assert_eq!(c.state().player(YOU).life, 10);
    let dealt: Vec<_> =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s,
                    target: Target::Player(YOU),
                    amount,
                    ..
                } if s == source => Some(amount),
                _ => None,
            })
            .collect();
    assert_eq!(
        dealt,
        [10],
        "one final source/recipient damage event, despite separate replacement histories"
    );
}
#[test]
fn lineage_unmodified_original_converges_after_scoped_half() {
    let (mut g, source) = scoped_converging(&SCOPED_HALVER);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 4),
            (source, Target::Player(THEM), 4)
        ]),
        [2, 2]
    );
    assert_eq!(c.state().player(YOU).life, 16);
    let dealt: Vec<_> =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s,
                    target: Target::Player(YOU),
                    amount,
                    ..
                } if s == source => Some(amount),
                _ => None,
            })
            .collect();
    assert_eq!(dealt, [4]);
}
#[test]
fn lineage_scope_without_redirect_keeps_independent_recipients() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    put(&mut g, &SCOPED_DOUBLER, YOU);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 2),
            (source, Target::Player(THEM), 3)
        ]),
        [4, 3]
    );
    assert_eq!(
        (c.state().player(YOU).life, c.state().player(THEM).life),
        (16, 17)
    );
}

fn half_prevented(_: &dyn Query, _: &Asker, d: &DamageEvent) -> u32 {
    d.amount / 2 + d.amount % 2
}
// The printed prevention clause of Gisela; its unrelated opponent-doubling clause is outside this fixture.
static FRACTION_PREVENTION: CardDef = CardDef {
    name: "Damage lineage event-relative half prevention",
    preventions: &[PreventionDef {
        hint: "prevent half, rounded up",
        applies: armor_to_you,
        amount: PreventionAmount::Relative(half_prevented),
    }],
    ..UNLIFE
};
#[test]
fn lineage_fraction_prevention_must_not_be_classified_as_fixed_specific_allowance() {
    let (mut g, source) = scoped_converging(&FRACTION_PREVENTION);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 4),
            (source, Target::Player(THEM), 4)
        ]),
        [2, 2]
    );
    assert_eq!(c.state().player(YOU).life, 16);
}
#[test]
fn lineage_fraction_prevention_without_redirect_keeps_its_scope() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    put(&mut g, &FRACTION_PREVENTION, YOU);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 4),
            (source, Target::Player(THEM), 4)
        ]),
        [2, 4]
    );
}

fn four_or_more(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p,Proposal::Damage(d) if d.amount>=4)
}
fn three(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = 3;
    }
}
// Printed Divine Presence: this candidate has not applied to either independent original fragment.
static PRESENCE: CardDef = CardDef {
    name: "Damage lineage Divine Presence",
    replacements: &[replacement(
        ReplacementKind::Damage,
        four_or_more,
        three,
        "four or more becomes three",
    )],
    ..UNLIFE
};
// Printed Ghosts of the Innocent replacement clause; the recipient chooses its order with redirection.
static GHOSTS: CardDef = CardDef {
    name: "Damage lineage Ghosts of the Innocent",
    replacements: &[replacement(
        ReplacementKind::Damage,
        any_damage,
        half,
        "half damage, rounded down",
    )],
    ..UNLIFE
};
#[test]
fn lineage_untouched_divine_presence_meets_combined_quantity_after_redirect() {
    let (mut g, source) = scoped_converging(&PRESENCE);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 2),
            (source, Target::Player(THEM), 2)
        ]),
        [1, 2]
    );
    let dealt: Vec<_> =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s, amount, ..
                } if s == source => Some(amount),
                _ => None,
            })
            .collect();
    assert_eq!(dealt, [3]);
}
#[test]
fn lineage_ghosts_global_half_still_applies_to_redirected_original() {
    let (mut g, source) = scoped_converging(&GHOSTS);
    let mut c = cx(&mut g, source);
    // Ghosts was put before the redirect: originalYOU has only Ghosts; originalTHEM chooses redirect before Ghosts.
    answers(&mut c, vec![Answer::Index(1)]);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 4),
            (source, Target::Player(THEM), 4)
        ]),
        [2, 2]
    );
}

fn armor_three(_: &dyn Query, _: &Asker, _: &DamageEvent) -> u32 {
    3
}
static ARMOR_THREE: CardDef = CardDef {
    name: "Damage lineage specific three",
    preventions: &[PreventionDef {
        hint: "prevent three",
        applies: armor_to_you,
        amount: PreventionAmount::Specific(armor_three),
    }],
    ..UNLIFE
};
#[test]
fn lineage_specific_allowance_keeps_unspent_capacity_after_original_is_zero() {
    let (mut g, source) = scoped_converging(&ARMOR_THREE);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 1),
            (source, Target::Player(THEM), 4)
        ]),
        [0, 2]
    );
    assert_eq!(c.state().player(YOU).life, 18);
    let prevented: u32 =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::DamagePrevented { amount, .. } => Some(amount),
                _ => None,
            })
            .sum();
    assert_eq!(prevented, 3);
}
#[test]
fn lineage_relative_rounding_uses_combined_untouched_quantity() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    put(&mut g, &FRACTION_PREVENTION, YOU);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 1),
            (source, Target::Player(YOU), 1)
        ]),
        [0, 1]
    );
    assert_eq!(c.state().player(YOU).life, 19);
}
#[test]
fn lineage_relative_rounding_keeps_prior_and_new_opportunities_separate() {
    let (mut g, source) = scoped_converging(&FRACTION_PREVENTION);
    let mut c = cx(&mut g, source);
    assert_eq!(
        c.damage_together(&[
            (source, Target::Player(YOU), 3),
            (source, Target::Player(THEM), 3)
        ]),
        [1, 1]
    );
}
fn nested_same(g: &mut dyn Rules, _: &Asker, before: &Proposal, _: &Proposal) {
    if let Proposal::Damage(d) = before {
        g.damage(d.source, d.target, d.amount);
    }
}
static NESTED_SAME_SOURCE: CardDef = CardDef {
    name: "Damage lineage separate imperative event",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        unchanged,
        "also deals separate damage",
    )
    .then(nested_same)],
    ..UNLIFE
};
fn pure_two(_: &dyn Query, _: &Asker, b: &DamageEvent, _: &DamageEvent) -> Vec<DamageComponent> {
    vec![DamageComponent::Additional(DamageEvent { amount: 2, ..*b })]
}
static PURE_SAME_SOURCE: CardDef = CardDef {
    name: "Damage lineage same-event additional two",
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        unchanged,
        "also deals two simultaneously",
    )
    .with_damage(pure_two)],
    ..UNLIFE
};
#[test]
fn lineage_nested_native_call_gets_fresh_specific_allowance() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    put(&mut g, &URZA_ARMOR, YOU);
    put(&mut g, &NESTED_SAME_SOURCE, YOU);
    let mut c = cx(&mut g, source);
    answers(&mut c, vec![Answer::Index(0)]);
    assert_eq!(c.damage(source, Target::Player(YOU), 3), 2);
    assert_eq!(c.state().player(YOU).life, 17);
    let amounts: Vec<_> =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s, amount, ..
                } if s == source => Some(amount),
                _ => None,
            })
            .collect();
    assert_eq!(amounts, [1, 2]);
    assert!(c.damage.is_none());
    println!("lineage layout Event={} Record={} PreventionDef={} PreventionAmount={} Cx={} context_pointer={}", std::mem::size_of::<Event>(), std::mem::size_of::<Record>(), std::mem::size_of::<PreventionDef>(), std::mem::size_of::<PreventionAmount>(), std::mem::size_of::<Cx<'_>>(), std::mem::size_of::<Option<Box<crate::replace::DamageContext>>>());
}
#[test]
fn lineage_pure_same_source_components_share_specific_allowance() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    put(&mut g, &URZA_ARMOR, YOU);
    put(&mut g, &PURE_SAME_SOURCE, YOU);
    let mut c = cx(&mut g, source);
    answers(&mut c, vec![Answer::Index(0)]);
    assert_eq!(c.damage(source, Target::Player(YOU), 3), 2);
    assert_eq!(c.state().player(YOU).life, 16);
    let amounts: Vec<_> =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage {
                    source: s, amount, ..
                } if s == source => Some(amount),
                _ => None,
            })
            .collect();
    assert_eq!(amounts, [4]);
    assert!(c.damage.is_none());
}
fn capacity(q: &dyn Query, a: &Asker, _: &DamageEvent) -> u32 {
    1 + q.obj(a.source).counters(CounterKind::Charge).max(0) as u32
}
fn add_capacity(g: &mut dyn Rules, a: &Asker, _: &Proposal, _: &Proposal) {
    g.add_counters(a.source, CounterKind::Charge, 2);
}
static STATE_CAP: CardDef = CardDef {
    name: "Damage lineage state-computed specific capacity",
    preventions: &[PreventionDef {
        hint: "prevent one plus charges",
        applies: armor_to_you,
        amount: PreventionAmount::Specific(capacity),
    }],
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_to_you,
        unchanged,
        "put two charge counters on this",
    )
    .then(add_capacity)],
    ..UNLIFE
};
#[test]
fn lineage_specific_capacity_is_state_computed_not_guessed_constant() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    let cap = put(&mut g, &STATE_CAP, YOU);
    let mut c = cx(&mut g, source);
    answers(&mut c, vec![Answer::Index(1)]);
    assert_eq!(c.damage(source, Target::Player(YOU), 4), 3);
    assert_eq!(c.obj(cap).counters(CounterKind::Charge), 2);
    assert_eq!(c.state().player(YOU).life, 17);
}

static STATE_CAP_ONLY: CardDef = CardDef {
    name: "Damage lineage state capacity without other modifier",
    replacements: &[],
    ..STATE_CAP
};
fn zero_capacity(q: &dyn Query, a: &Asker, _: &DamageEvent) -> u32 {
    q.obj(a.source).counters(CounterKind::Charge).max(0) as u32
}
static STATE_CAP_ZERO: CardDef = CardDef {
    name: "Damage lineage zero initial state capacity",
    preventions: &[PreventionDef {
        hint: "prevent charges",
        applies: armor_to_you,
        amount: PreventionAmount::Specific(zero_capacity),
    }],
    ..STATE_CAP
};
#[test]
fn lineage_specific_capacity_recomputes_on_fresh_native_event() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    let cap = put(&mut g, &STATE_CAP_ONLY, YOU);
    let mut c = cx(&mut g, source);
    assert_eq!(c.damage(source, Target::Player(YOU), 4), 3);
    c.doer = Some(YOU);
    c.add_counters(cap, CounterKind::Charge, 2);
    c.end_sentence();
    c.doer = None;
    assert_eq!(c.damage(source, Target::Player(YOU), 4), 1);
    assert_eq!(c.state().player(YOU).life, 16);
    assert!(c.damage.is_none());
}
#[test]
fn lineage_specific_capacity_zero_first_application_does_not_refill() {
    let mut g = game();
    let source = put(&mut g, &BEAR, YOU);
    let cap = put(&mut g, &STATE_CAP_ZERO, YOU);
    let mut c = cx(&mut g, source);
    answers(&mut c, vec![Answer::Index(1)]);
    assert_eq!(c.damage(source, Target::Player(YOU), 4), 4);
    assert_eq!(c.obj(cap).counters(CounterKind::Charge), 2);
    assert_eq!(c.state().player(YOU).life, 16);
}

#[test]
fn lineage_player_source_order_survives_each_quantity_modifier() {
    let mut g = game();
    let a = put(&mut g, &BEAR, YOU);
    let b = put(&mut g, &BEAR, YOU);
    put(&mut g, &DOUBLER, YOU);
    let salve = put(&mut g, &SHIELD, YOU);
    let mut c = cx(&mut g, a);
    shield(&mut c, salve, Target::Player(YOU), 3);
    answers(
        &mut c,
        vec![Answer::Cards(vec![b]), Answer::Index(0), Answer::Index(0)],
    );
    assert_eq!(
        c.damage_together(&[(a, Target::Player(YOU), 1), (b, Target::Player(YOU), 1)]),
        [1, 0]
    );
    let prevented: Vec<_> =
        c.g.log
            .iter()
            .filter_map(|r| match r.event {
                Event::DamagePrevented { source, amount, .. } => Some((source, amount)),
                _ => None,
            })
            .collect();
    assert_eq!(prevented, [(b, 2), (a, 1)]);
}
#[test]
fn lineage_unasked_remaining_sources_are_chosen_if_another_prevention_saves_the_shield() {
    let mut g = game();
    let a = put(&mut g, &BEAR, YOU);
    let b = put(&mut g, &BEAR, YOU);
    let c_source = put(&mut g, &BEAR, YOU);
    let d = put(&mut g, &BEAR, YOU);
    put(&mut g, &FRACTION_PREVENTION, YOU);
    let salve = put(&mut g, &SHIELD, YOU);
    let mut c = cx(&mut g, a);
    shield(&mut c, salve, Target::Player(YOU), 3);
    answers(
        &mut c,
        vec![
            Answer::Cards(vec![a]),
            Answer::Cards(vec![b]),
            Answer::Index(0),
            Answer::Index(0),
            Answer::Cards(vec![d]),
            Answer::Index(0),
        ],
    );
    assert_eq!(
        c.damage_together(&[
            (a, Target::Player(YOU), 2),
            (b, Target::Player(YOU), 2),
            (c_source, Target::Player(YOU), 2),
            (d, Target::Player(YOU), 2)
        ]),
        [0, 0, 1, 0]
    );
}

// A threshold applies to the same matched damage amount carried by the trigger, not to the legacy data slots.
static THRESHOLD_PROBE: CardDef = CardDef {
    name: "Damage exact threshold probe",
    abilities: &[triggered(when(EventMask::DAMAGE, player_damage), gain_quantity, "Whenever at least five damage is dealt, gain that much life.").damage_amount().sums_at_least(5)],
    ..BEAR
};

#[test]
fn exact_damage_threshold_uses_the_matched_batch_sum() {
    for (amounts, expected) in [([3, 3], Some(6)), ([2, 2], None)] {
        let mut g = game();
        let source = put(&mut g, &SOURCE, YOU);
        let watcher = put(&mut g, &THRESHOLD_PROBE, YOU);
        let damage = |amount| Event::Damage {
            source, target: Target::Player(THEM), amount, kind: DamageKind::new(false, ColorSet::of(Color::Red)),
            controller: YOU, recipient_excess: 0, known: u32::MAX,
        };
        cx(&mut g, source).emit_batch(&[damage(amounts[0]), damage(amounts[1])], None);
        let seen: Vec<_> = g.state.triggers.iter().filter(|t| t.source == watcher).map(|t| t.event_amount).collect();
        assert_eq!(seen, expected.into_iter().map(Some).collect::<Vec<_>>());
    }
}

// Shared enchanted-damage matcher keeps legacy data representable while exact capture remains independent.
static ENCHANTED_DAMAGE_PROBE: CardDef = CardDef {
    name: "Enchanted damage amount probe", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(cantrip_mechanics::triggers::ENCHANTED_CREATURE_DEALT_DAMAGE, gain_quantity,
        "Whenever enchanted creature is dealt damage, gain that much life.").damage_amount()],
    ..CardDef::BLANK
};

#[test]
fn enchanted_damage_trigger_keeps_legacy_and_exact_amounts() {
    for amount in [7_u32, u32::MAX] {
        let mut g = game();
        let source = put(&mut g, &SOURCE, YOU);
        let bear = put(&mut g, &BEAR, YOU);
        let aura = put(&mut g, &ENCHANTED_DAMAGE_PROBE, YOU);
        g.state.update_obj(aura, |o| o.attached_to = Some(bear));
        let event = Event::Damage { source, target: Target::Object(bear), amount,
            kind: DamageKind::new(false, ColorSet::of(Color::Red)), controller: YOU, recipient_excess: 0, known: u32::MAX };
        cx(&mut g, source).emit_batch(&[event], None);
        let trigger = g.state.triggers.iter().find(|t| t.source == aura).expect("enchanted damage triggered");
        assert_eq!(trigger.event_amount, Some(amount));
        assert_eq!(trigger.data[0], amount);
    }
}

static SHARED_DAMAGE_PROBE: CardDef = CardDef {
    name: "Shared damage amount probe", types: TypeSet::of(&[CardType::Creature]),
    abilities: &[triggered(cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, gain_quantity,
        "Whenever this deals combat damage, gain that much life.").damage_amount()],
    ..CardDef::BLANK
};

#[test]
fn shared_damage_matchers_preserve_legacy_payload_and_exact_channel() {
    use cantrip_mechanics::triggers::{this_dealt_damage, equipped_creature_deals_combat_damage_to_player, deals_combat_damage_to_player_subject};
    let mut g = game();
    let source = put(&mut g, &SHARED_DAMAGE_PROBE, YOU);
    let equipment = put(&mut g, &ENCHANTED_DAMAGE_PROBE, YOU);
    g.state.update_obj(equipment, |o| o.attached_to = Some(source));
    for amount in [7_u32, u32::MAX] {
        let combat = Event::Damage { source, target: Target::Player(THEM), amount,
            kind: DamageKind::new(true, ColorSet::of(Color::Red)), controller: YOU, recipient_excess: 0, known: u32::MAX };
        let hurt = Event::Damage { source, target: Target::Object(source), amount,
            kind: DamageKind::new(true, ColorSet::of(Color::Red)), controller: YOU, recipient_excess: 0, known: u32::MAX };
        let view = cx(&mut g, source);
        let expected = amount;
        assert_eq!(this_dealt_damage(&view, source, &hurt).unwrap().data[0], expected);
        assert_eq!(deals_combat_damage_to_player_subject(&view, source, &combat).unwrap().data[0], expected);
        assert_eq!(equipped_creature_deals_combat_damage_to_player(&view, equipment, &combat).unwrap().data[0], expected);
        drop(view);
        cx(&mut g, source).emit_batch(&[combat], None);
        let triggered = g.state.triggers.iter().rev().find(|t| t.source == source).expect("combat damage triggered");
        assert_eq!(triggered.data[0], expected);
        assert_eq!(triggered.event_amount, Some(amount));
    }
}

#[test]
fn printed_metropolis_reformer_gains_life_from_large_damage() {
    let reformer = cantrip_cards::all().into_iter().find(|c| c.name == "Metropolis Reformer").unwrap();
    for (amount, expected_life) in [(7_u32, 27), (1_000_000, 1_000_020)] {
        let cards = Box::leak(Box::new(CardTable::new([reformer])));
        let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(reformer); 20]; 2], ..Default::default() });
        let creature = put(&mut g, reformer, YOU);
        let source = creature;
        let event = Event::Damage { source, target: Target::Object(creature), amount,
            kind: DamageKind::new(false, ColorSet::NONE), controller: YOU, recipient_excess: 0, known: u32::MAX };
        cx(&mut g, source).emit_batch(&[event], None);
        let trigger = *g.state.triggers.iter().find(|t| t.source == creature).unwrap();
        assert_eq!(trigger.event_amount, Some(amount));
        // Invoke the printed ability with the engine's captured trigger payload.
        let item = StackItem {
            instance: AbilityAcquisition::OWN, event_amount: trigger.event_amount,
            kind: StackKind::Triggered, object: trigger.source, ability: trigger.ability,
            controller: trigger.controller, inline_targets: [Target::None; cantrip_state::INLINE_TARGETS],
            target_count: 0, inline_spec_of: [0; cantrip_state::INLINE_TARGETS], target_choosers: Default::default(), x: 0, kicked: 0,
            counted: 0, paid: 0, modes: 0, way: 0, choice: 0, subject: trigger.subject,
            data: trigger.data, members: trigger.members, serial: 0, gift: None,
            shares: StackItem::NO_SHARES, spill: 0, exile_instead: ExileInstead::Never,
            flashback: false, attacking: false, since: 0, tapped_since: 0, copied: false,
            cast_known: u32::MAX, acquired: trigger.acquired, words: Words::NONE,
        };
        let AbilityDef::Triggered(ability) = reformer.abilities[0] else { panic!("expected printed trigger") };
        (ability.effect)(&mut cx(&mut g, creature), &item).unwrap();
        assert_eq!(g.state.player(YOU).life, expected_life);
    }
}

#[test]
fn printed_bloodthirsty_conqueror_gains_life_from_opponents_loss() {
    let conqueror = cantrip_cards::all().into_iter().find(|c| c.name == "Bloodthirsty Conqueror").unwrap();
    for (amount, expected_life) in [(7_u32, 27), (1_000_000, 1_000_020)] {
        let cards = Box::leak(Box::new(CardTable::new([conqueror])));
        let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(conqueror); 20]; 2], ..Default::default() });
        let creature = put(&mut g, conqueror, YOU);
        let source = creature;
        let event = Event::LifeChange { player: THEM, delta: if amount == i32::MAX as u32 + 1 { i32::MIN } else { -(amount as i32) } };
        cx(&mut g, source).emit_batch(&[event], None);
        let trigger = *g.state.triggers.iter().find(|t| t.source == creature).unwrap();
        assert_eq!(trigger.data[0], amount);
        // Invoke the printed ability with the engine's captured trigger payload.
        let item = StackItem {
            instance: AbilityAcquisition::OWN, event_amount: trigger.event_amount,
            kind: StackKind::Triggered, object: trigger.source, ability: trigger.ability,
            controller: trigger.controller, inline_targets: [Target::None; cantrip_state::INLINE_TARGETS],
            target_count: 0, inline_spec_of: [0; cantrip_state::INLINE_TARGETS], target_choosers: Default::default(), x: 0, kicked: 0,
            counted: 0, paid: 0, modes: 0, way: 0, choice: 0, subject: trigger.subject,
            data: trigger.data, members: trigger.members, serial: 0, gift: None,
            shares: StackItem::NO_SHARES, spill: 0, exile_instead: ExileInstead::Never,
            flashback: false, attacking: false, since: 0, tapped_since: 0, copied: false,
            cast_known: u32::MAX, acquired: trigger.acquired, words: Words::NONE,
        };
        let AbilityDef::Triggered(ability) = conqueror.abilities[0] else { panic!("expected printed trigger") };
        (ability.effect)(&mut cx(&mut g, creature), &item).unwrap();
        assert_eq!(g.state.player(YOU).life, expected_life);
    }
}

#[test]
fn printed_stream_of_life_gains_life_for_large_x() {
    let stream = cantrip_cards::all().into_iter().find(|c| c.name == "Stream of Life").unwrap();
    for (x, expected_life) in [(7_u32, 27), (1_000_000, 1_000_020)] {
        let cards = Box::leak(Box::new(CardTable::new([stream])));
        let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(stream); 20]; 2], ..Default::default() });
        let spell = g.state.push_obj(Obj::new(cards.id_of(stream), YOU, Zone::STACK));
        g.state.zone_push(Zone::STACK, spell);
        let mut item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(cards.id_of(stream)), YOU, &[Target::Player(YOU)]);
        item.x = x;
        (stream.spell.unwrap().effect)(&mut cx(&mut g, spell), &item).unwrap();
        assert_eq!(g.state.player(YOU).life, expected_life);
    }
}

// A replacement whose condition does not ask how much damage there is: it may be judged on a part of a converged event
// that an earlier prevention left at 0.
fn any_damage_at_all(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(_))
}
static PLAIN_DOUBLER: CardDef = CardDef {
    name: "Damage convergence plain doubler",
    replacements: &[replacement(ReplacementKind::Damage, any_damage_at_all, double, "double damage")],
    ..UNLIFE
};

/// The part of a converged event an effect is still eligible for may be damage a prevention effect already took to 0, while
/// the part redirected there is not: the effect chosen for the whole meets nothing left of its own part, does nothing,
/// and the event goes on (CR 615.1, 614.5) - never a panic. Every order of the choices is played.
#[test]
fn an_effect_chosen_for_a_part_prevented_to_zero_does_nothing_and_the_event_goes_on() {
    for first in 0..2 {
        for second in 0..2 {
            for third in 0..2 {
                let mut g = game();
                let source = put(&mut g, &BEAR, YOU);
                put(&mut g, &PREVENT_ALL, YOU);
                put(&mut g, &REDIRECT_OTHER_TO_YOU, YOU);
                put(&mut g, &PLAIN_DOUBLER, YOU);
                let mut c = cx(&mut g, source);
                answers(&mut c, vec![Answer::Index(first), Answer::Index(second), Answer::Index(third), Answer::Index(0), Answer::Index(0)]);
                let dealt = c.damage_together(&[(source, Target::Player(YOU), 2), (source, Target::Player(THEM), 2)]);
                // Everything ends up dealt to YOU, whom PREVENT_ALL protects from all of it.
                assert_eq!(dealt, [0, 0], "answers {first} {second} {third}");
                assert_eq!(c.state().player(YOU).life, 20);
                assert_eq!(c.state().player(THEM).life, 20);
            }
        }
    }
}
