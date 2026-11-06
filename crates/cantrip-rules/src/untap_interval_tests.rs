//! CR 502.3, 701.26b and 611.2b, over forced positions and test definitions.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn blocks_when_untapped(q: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    id != a.source && !q.obj(a.source).tapped
}
fn holds(q: &dyn Query, e: &ContinuousEffect) -> bool {
    is_live(q, e.source) && q.obj(e.source).zone == Zone::BATTLEFIELD && q.obj(e.source).tapped
}
fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    t.object().is_some_and(|id| {
        is_live(q, id) && q.obj(id).zone == Zone::BATTLEFIELD && q.chars(id).is(CardType::Creature)
    })
}
fn pump(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(
        item.object,
        item.target(0).object().unwrap(),
        EffectKind::Pump(2, 0),
        Duration::WhileSourceTapped,
    );
    Ok(())
}
static HOLDER: CardDef = CardDef {
    name: "Interval Holder",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[ability(
        TAP,
        pump,
        "{T}: Target creature gets +2/+0 for as long as this remains tapped.",
    )
    .targets(&[target(creature)])],
    lasting: &[Lasting { text: "", holds }],
    ..CardDef::BLANK
};
static LOCK: CardDef = CardDef {
    name: "Untap Lock",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    cant_become_untapped: &[blocks_when_untapped],
    ..CardDef::BLANK
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &HOLDER,
        &LOCK,
        &WATCHER,
        &CONTROL_LOCK,
        &PAID_HOLDER,
        &TAP_WATCHER,
        &MANA_HOLDER,
        &RESTART_MANA_HOLDER,
    ])));
    Game::new(
        cards,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    )
}
fn put(g: &mut Game, card: &'static CardDef, player: PlayerId, tapped: bool) -> ObjectId {
    let id = g
        .state
        .push_obj(Obj::new(g.cards.id_of(card), player, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.update_obj(id, |o| o.tapped = tapped);
    g.state.see_statics();
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
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
fn untap_step_judges_all_prohibitions_before_any_permanent_untaps() {
    let mut g = game();
    let lock = put(&mut g, &LOCK, YOU, true);
    let other = put(&mut g, &HOLDER, YOU, true);
    crate::turn::untap_step(&mut cx(&mut g), YOU).unwrap();
    assert!(!g.state.obj(lock).tapped);
    assert!(
        !g.state.obj(other).tapped,
        "a simultaneous untap must not see the other permanent's new status"
    );
}
#[test]
fn untap_and_retap_before_resolution_cannot_restart_the_first_abilitys_duration() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let mut item = crate::actions::stack_item(
        StackKind::Activated,
        source,
        AbilityRef {
            card: g.cards.id_of(&HOLDER),
            index: 0,
        },
        YOU,
        &[Target::Object(target)],
    );
    item.since = g.log.len() as u32;
    g.state.stack_push(item);
    crate::ops::untap(&mut cx(&mut g), source);
    crate::ops::tap(&mut cx(&mut g), source);
    let serial = g.state.stack[0].serial;
    let mut frame = Frame::new(FrameKind::Resolve { serial });
    frame.log_mark = g.log.len() as u32;
    g.state.frame_push(frame);
    let mut cx = cx(&mut g);
    cx.frame = 0;
    crate::resolve::resolve(&mut cx).unwrap();
    assert_eq!(
        cx.chars(target).power,
        Some(2),
        "the first tapped interval ended before resolution"
    );
}

fn interval(g: &mut Game, source: ObjectId) -> u32 {
    let mut item = crate::actions::stack_item(
        StackKind::Activated,
        source,
        AbilityRef {
            card: g.cards.id_of(&HOLDER),
            index: 0,
        },
        YOU,
        &[],
    );
    item.since = g.log.len() as u32;
    item.tapped_since = item.since;
    g.state.stack_push(item);
    g.state.stack.last().unwrap().serial
}
fn begin_resolution(g: &mut Game, serial: u32) -> usize {
    let mut frame = Frame::new(FrameKind::Resolve { serial });
    frame.log_mark = g.log.len() as u32;
    let at = g.state.agenda.len();
    g.state.frame_push(frame);
    at
}
fn apply(g: &mut Game, frame: usize, source: ObjectId, target: ObjectId, duration: Duration) {
    let mut cx = cx(g);
    cx.frame = frame;
    cantrip_api::Game::add_effect(&mut cx, source, target, EffectKind::Pump(2, 0), duration);
}
fn control(g: &mut Game, source: ObjectId, player: PlayerId) {
    cantrip_api::Game::add_effect(
        &mut cx(g),
        source,
        source,
        EffectKind::Control(player),
        Duration::Indefinite,
    );
}
fn untap_events(g: &Game) -> Vec<ObjectId> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::Untapped { permanent } => Some(permanent),
            _ => None,
        })
        .collect()
}

#[test]
fn an_unbroken_source_interval_begins_and_ends_at_the_actual_untap() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let serial = interval(&mut g, source);
    let frame = begin_resolution(&mut g, serial);
    let reads = cantrip_api::history::reads_total();
    apply(&mut g, frame, source, target, Duration::WhileSourceTapped);
    assert!(
        cantrip_api::history::reads_total() > reads,
        "the raw core history read must inform loop detection"
    );
    assert_eq!(g.chars(target).power, Some(4));
    crate::ops::untap(&mut cx(&mut g), source);
    assert_eq!(g.chars(target).power, Some(2));
    crate::ops::tap(&mut cx(&mut g), source);
    assert_eq!(
        g.chars(target).power,
        Some(2),
        "an expired effect cannot revive"
    );
}

#[test]
fn retapping_during_resolution_restarts_a_duration_broken_before_resolution() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let serial = interval(&mut g, source);
    crate::ops::untap(&mut cx(&mut g), source);
    let frame = begin_resolution(&mut g, serial);
    crate::ops::tap(&mut cx(&mut g), source);
    apply(&mut g, frame, source, target, Duration::WhileSourceTapped);
    assert_eq!(
        g.chars(target).power,
        Some(4),
        "CR 611.2b permits a new interval during resolution"
    );
}

#[test]
fn control_lost_and_regained_before_resolution_breaks_only_the_controlled_duration() {
    for controlled in [false, true] {
        let mut g = game();
        let source = put(&mut g, &HOLDER, YOU, true);
        let target = put(&mut g, &HOLDER, THEM, false);
        let serial = interval(&mut g, source);
        control(&mut g, source, THEM);
        control(&mut g, source, YOU);
        let frame = begin_resolution(&mut g, serial);
        apply(
            &mut g,
            frame,
            source,
            target,
            if controlled {
                Duration::WhileSourceTappedAndControlled
            } else {
                Duration::WhileSourceTapped
            },
        );
        assert_eq!(g.chars(target).power, Some(if controlled { 2 } else { 4 }));
    }
}

#[test]
fn a_conjunction_restarts_only_when_its_entire_condition_returns_during_resolution() {
    for restore_tap in [false, true] {
        for restore_control in [false, true] {
            let mut g = game();
            let source = put(&mut g, &HOLDER, YOU, true);
            let target = put(&mut g, &HOLDER, THEM, false);
            let serial = interval(&mut g, source);
            crate::ops::untap(&mut cx(&mut g), source);
            control(&mut g, source, THEM);
            let frame = begin_resolution(&mut g, serial);
            if restore_tap {
                crate::ops::tap(&mut cx(&mut g), source);
            }
            if restore_control {
                control(&mut g, source, YOU);
            }
            apply(
                &mut g,
                frame,
                source,
                target,
                Duration::WhileSourceTappedAndControlled,
            );
            assert_eq!(
                g.chars(target).power,
                Some(if restore_tap && restore_control { 4 } else { 2 })
            );
        }
    }
}

#[test]
fn a_restart_followed_by_another_lapse_does_not_begin_an_effect() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let serial = interval(&mut g, source);
    crate::ops::untap(&mut cx(&mut g), source);
    let frame = begin_resolution(&mut g, serial);
    crate::ops::tap(&mut cx(&mut g), source);
    control(&mut g, source, THEM);
    apply(
        &mut g,
        frame,
        source,
        target,
        Duration::WhileSourceTappedAndControlled,
    );
    assert_eq!(g.chars(target).power, Some(2));
}

#[test]
fn a_copy_starts_its_own_interval_and_does_not_repair_the_original() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let original = interval(&mut g, source);
    crate::ops::untap(&mut cx(&mut g), source);
    crate::ops::tap(&mut cx(&mut g), source);
    let copied = crate::ops::copy_ability(&mut cx(&mut g), Target::Ability(original), YOU, false)
        .unwrap()
        .unwrap();
    let copied_frame = begin_resolution(&mut g, copied);
    apply(
        &mut g,
        copied_frame,
        source,
        target,
        Duration::WhileSourceTapped,
    );
    assert_eq!(g.chars(target).power, Some(4));
    let original_frame = begin_resolution(&mut g, original);
    apply(
        &mut g,
        original_frame,
        source,
        target,
        Duration::WhileSourceTapped,
    );
    assert_eq!(
        g.chars(target).power,
        Some(4),
        "the original still does nothing"
    );
}

#[test]
fn stun_replacement_and_prohibition_do_not_emit_untapped_or_break_a_tapped_interval() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let serial = interval(&mut g, source);
    crate::counters::put(
        &mut cx(&mut g),
        source,
        CounterKind::Stun,
        2,
        YOU,
        crate::counters::How::Effect,
    );
    crate::ops::untap(&mut cx(&mut g), source);
    assert!(g.state.obj(source).tapped);
    assert_eq!(g.state.obj(source).counters(CounterKind::Stun), 1);
    assert!(untap_events(&g).is_empty());
    let frame = begin_resolution(&mut g, serial);
    apply(&mut g, frame, source, target, Duration::WhileSourceTapped);
    assert_eq!(g.chars(target).power, Some(4));
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        source,
        source,
        EffectKind::CantBecomeUntapped,
        Duration::EndOfTurn,
    );
    crate::ops::untap(&mut cx(&mut g), source);
    assert_eq!(
        g.state.obj(source).counters(CounterKind::Stun),
        1,
        "a prohibited untap removes no stun counter"
    );
    assert!(untap_events(&g).is_empty());
    assert_eq!(g.chars(target).power, Some(4));
}

#[test]
fn can_untap_means_payable_before_replacements_and_excludes_step_only_bans() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    assert!(g.can_untap(source));
    crate::counters::put(
        &mut cx(&mut g),
        source,
        CounterKind::Stun,
        1,
        YOU,
        crate::counters::How::Effect,
    );
    assert!(g.can_untap(source), "CR 118.11: a replaced cost is paid");
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        source,
        source,
        EffectKind::DoesntUntap,
        Duration::EndOfTurn,
    );
    assert!(
        g.can_untap(source),
        "an untap-step ban does not prohibit an untap cost"
    );
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        source,
        source,
        EffectKind::CantBecomeUntapped,
        Duration::EndOfTurn,
    );
    assert!(!g.can_untap(source));
    let untapped = put(&mut g, &HOLDER, YOU, false);
    assert!(!g.can_untap(untapped));
    crate::ops::move_to(&mut cx(&mut g), source, Zone::graveyard(YOU));
    assert!(
        !g.can_untap(source),
        "a stale object is not a payable untap"
    );
}

fn untapped_match(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Untapped { permanent } => Fired::about(Target::Object(*permanent)),
        _ => None,
    }
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}
static WATCHER: CardDef = CardDef {
    name: "Untap Batch Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(
        when(EventMask::UNTAPPED, untapped_match),
        nothing,
        "Whenever one or more permanents become untapped, do nothing.",
    )
    .once_per_batch()],
    ..CardDef::BLANK
};
#[test]
fn simultaneous_untaps_are_one_trigger_batch_with_one_event_per_changed_permanent() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU, false);
    let a = put(&mut g, &HOLDER, YOU, true);
    let b = put(&mut g, &HOLDER, YOU, true);
    let c = put(&mut g, &HOLDER, YOU, true);
    crate::counters::put(
        &mut cx(&mut g),
        c,
        CounterKind::Stun,
        2,
        YOU,
        crate::counters::How::Effect,
    );
    cantrip_api::Game::untap_together(&mut cx(&mut g), &[a, b, c, a, c, watcher]);
    assert_eq!(untap_events(&g), [a, b]);
    assert_eq!(
        g.state.obj(c).counters(CounterKind::Stun),
        1,
        "duplicate ids do not pay multiple stun replacements"
    );
    let triggers: Vec<_> = g
        .state
        .triggers
        .iter()
        .filter(|t| t.source == watcher)
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(g.state.members(triggers[0].members), [a, b]);
    crate::ops::tap(&mut cx(&mut g), a);
    crate::ops::untap(&mut cx(&mut g), a);
    assert_eq!(
        g.state
            .triggers
            .iter()
            .filter(|t| t.source == watcher)
            .count(),
        2
    );
}

fn blocks_when_yours(q: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    id != a.source && q.obj(a.source).controller == YOU
}
static CONTROL_LOCK: CardDef = CardDef {
    name: "Untap Control Lock",
    cant_become_untapped: &[blocks_when_yours],
    ..LOCK
};
#[test]
fn simultaneous_stun_removal_does_not_change_another_permanents_prohibition() {
    let mut g = game();
    let lock = put(&mut g, &CONTROL_LOCK, YOU, true);
    let free = put(&mut g, &HOLDER, YOU, true);
    crate::counters::put(
        &mut cx(&mut g),
        lock,
        CounterKind::Stun,
        1,
        YOU,
        crate::counters::How::Effect,
    );
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        lock,
        lock,
        EffectKind::Control(THEM),
        Duration::WhileCounter(CounterKind::Stun),
    );
    assert_eq!(g.state.obj(lock).controller, THEM);
    crate::ops::untap_together(&mut cx(&mut g), &[lock, free]);
    assert!(g.state.obj(lock).tapped);
    assert_eq!(g.state.obj(lock).counters(CounterKind::Stun), 0);
    assert_eq!(g.state.obj(lock).controller, YOU);
    assert!(
        !g.state.obj(free).tapped,
        "the counter's removal cannot change another member's planned untap"
    );
}

fn self_pump(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(
        item.object,
        item.object,
        EffectKind::Pump(2, 0),
        Duration::WhileSourceTapped,
    );
    Ok(())
}
fn untap_and_add(
    g: &mut dyn cantrip_api::Game,
    a: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    g.untap(a.source);
    g.add_mana(a.controller, Mana::C, 1);
    Ok(())
}
static PAID_HOLDER: CardDef = CardDef {
    name: "Paid Tapped Interval Holder",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(
            cost("{1}, {T}"),
            self_pump,
            "{1}, {T}: This gets +2/+0 for as long as it remains tapped.",
        ),
        mana_ability(
            TAP,
            add(Mana::C).by(untap_and_add),
            "{T}: Untap this. Add {C}.",
        ),
    ],
    ..HOLDER
};
#[test]
fn untapping_while_producing_mana_precedes_the_final_source_tap_cost() {
    let mut g = game();
    let source = put(&mut g, &PAID_HOLDER, YOU, false);
    let target = put(&mut g, &HOLDER, THEM, false);
    let mut frame = Frame::new(FrameKind::Act { player: YOU });
    for option in [
        ActionOption::on(ActionKind::ActivateAbility, source, 0),
        ActionOption::on(ActionKind::ActivateAbility, source, 1),
        ActionOption::simple(ActionKind::ConfirmCasting),
    ] {
        frame
            .answers
            .push((Some(YOU), Answer::Act(Action::of(&option))));
    }
    frame.answers.push((Some(YOU), Answer::CostPayment(1)));
    g.state.frame_push(frame);
    let mut activation = cx(&mut g);
    activation.frame = 0;
    crate::actions::act(&mut activation, YOU)
        .expect("the scripted activation and mana payment are legal");
    assert!(
        g.state.obj(source).tapped,
        "the outer activation paid its final {{T}} cost"
    );
    let item = *g
        .state
        .stack
        .iter()
        .find(|i| i.object == source && i.kind == StackKind::Activated)
        .unwrap();
    let untap = g
        .log
        .iter()
        .find(|r| matches!(r.event, Event::Untapped { permanent } if permanent == source))
        .unwrap();
    assert!(
        untap.serial >= item.since && untap.serial < item.tapped_since,
        "the cost's interval starts after the mana ability's untap"
    );
    let frame = begin_resolution(&mut g, item.serial);
    apply(&mut g, frame, source, target, Duration::WhileSourceTapped);
    assert_eq!(g.chars(target).power, Some(4));
}

#[test]
fn an_immediate_nested_mana_effect_does_not_borrow_an_outer_stack_interval() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, false);
    let serial = interval(&mut g, source);
    crate::ops::untap(&mut cx(&mut g), source);
    crate::ops::tap(&mut cx(&mut g), source);
    let frame = begin_resolution(&mut g, serial);
    let mut cx = cx(&mut g);
    cx.frame = frame;
    cx.mana_source = Some(source);
    cantrip_api::Game::add_effect(
        &mut cx,
        source,
        target,
        EffectKind::Pump(2, 0),
        Duration::WhileSourceTapped,
    );
    assert_eq!(cx.chars(target).power, Some(4));
}

#[test]
fn affected_tapped_and_counter_durations_still_end_on_their_own_conditions() {
    let mut g = game();
    let source = put(&mut g, &HOLDER, YOU, true);
    let target = put(&mut g, &HOLDER, THEM, true);
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        source,
        target,
        EffectKind::Pump(2, 0),
        Duration::WhileTapped,
    );
    crate::ops::untap(&mut cx(&mut g), source);
    assert_eq!(
        g.chars(target).power,
        Some(4),
        "WhileTapped names the affected object"
    );
    crate::ops::untap(&mut cx(&mut g), target);
    assert_eq!(g.chars(target).power, Some(2));
    crate::counters::put(
        &mut cx(&mut g),
        target,
        CounterKind::Stun,
        1,
        YOU,
        crate::counters::How::Effect,
    );
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        source,
        target,
        EffectKind::Pump(2, 0),
        Duration::WhileCounter(CounterKind::Stun),
    );
    crate::counters::remove(&mut cx(&mut g), target, CounterKind::Stun, 1);
    assert_eq!(g.chars(target).power, Some(2));
}

fn tapped_match(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Tapped { permanent } => Fired::about(Target::Object(*permanent)),
        _ => None,
    }
}
static TAP_WATCHER: CardDef = CardDef {
    name: "Tap Compatibility Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(
        when(EventMask::TAPPED, tapped_match),
        nothing,
        "Whenever a permanent becomes tapped, do nothing.",
    )],
    ..CardDef::BLANK
};
#[test]
fn tap_and_untap_masks_keep_history_counts_and_trigger_subscribers_separate() {
    let mut g = game();
    let tapper = put(&mut g, &TAP_WATCHER, YOU, false);
    let untapper = put(&mut g, &WATCHER, YOU, false);
    let source = put(&mut g, &HOLDER, YOU, true);
    crate::ops::untap(&mut cx(&mut g), source);
    assert_eq!(
        g.history()
            .count(EventQuery::kind(EventMask::TAPPED).object(source)),
        0
    );
    assert_eq!(
        g.history()
            .count(EventQuery::kind(EventMask::UNTAPPED).object(source)),
        1
    );
    assert_eq!(
        g.state
            .triggers
            .iter()
            .filter(|t| t.source == tapper)
            .count(),
        0
    );
    assert_eq!(
        g.state
            .triggers
            .iter()
            .filter(|t| t.source == untapper)
            .count(),
        1
    );
    crate::ops::tap(&mut cx(&mut g), source);
    assert_eq!(
        g.history()
            .count(EventQuery::kind(EventMask::TAPPED).object(source)),
        1
    );
    assert_eq!(
        g.history()
            .count(EventQuery::kind(EventMask::UNTAPPED).object(source)),
        1
    );
    assert_eq!(
        g.state
            .triggers
            .iter()
            .filter(|t| t.source == tapper)
            .count(),
        1
    );
    assert_eq!(
        g.state
            .triggers
            .iter()
            .filter(|t| t.source == untapper)
            .count(),
        1
    );
}

#[test]
fn a_source_that_is_untapped_or_has_left_cannot_begin_a_tapped_duration() {
    for leaves in [false, true] {
        let mut g = game();
        let source = put(&mut g, &HOLDER, YOU, leaves);
        let target = put(&mut g, &HOLDER, THEM, false);
        let serial = interval(&mut g, source);
        if leaves {
            crate::ops::move_to(&mut cx(&mut g), source, Zone::graveyard(YOU));
            let returned = put(&mut g, &HOLDER, YOU, true);
            assert_ne!(source, returned);
        }
        let frame = begin_resolution(&mut g, serial);
        apply(&mut g, frame, source, target, Duration::WhileSourceTapped);
        assert_eq!(g.chars(target).power, Some(2));
    }
}

#[test]
fn phasing_breaks_the_interval_and_only_a_restart_during_resolution_can_begin_again() {
    for during_resolution in [false, true] {
        let mut g = game();
        let source = put(&mut g, &HOLDER, YOU, true);
        let target = put(&mut g, &HOLDER, THEM, false);
        let serial = interval(&mut g, source);
        crate::phasing::phase_out(&mut cx(&mut g), &[source], None, false);
        assert!(
            !g.can_untap(source),
            "a phased-out source is not available to an untap cost"
        );
        let early = during_resolution.then(|| begin_resolution(&mut g, serial));
        crate::phasing::untap_step(&mut cx(&mut g), YOU);
        assert_eq!(g.state.obj(source).zone, Zone::BATTLEFIELD);
        assert!(g.state.obj(source).tapped);
        let frame = early.unwrap_or_else(|| begin_resolution(&mut g, serial));
        apply(&mut g, frame, source, target, Duration::WhileSourceTapped);
        assert_eq!(
            g.chars(target).power,
            Some(if during_resolution { 4 } else { 2 })
        );
    }
}

const CHARGE_X: Cost = Cost {
    variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::Charge)),
    ..Cost::NONE
};
fn inner_retap_mana(
    g: &mut dyn cantrip_api::Game,
    a: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    assert_eq!(
        g.mana_ability_x(),
        Some(1),
        "the nested activation has its independent X"
    );
    g.untap(a.source);
    g.tap(a.source);
    g.add_mana(a.controller, Mana::C, 1);
    Ok(())
}
fn inner_untap_mana(
    g: &mut dyn cantrip_api::Game,
    a: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    assert_eq!(g.mana_ability_x(), Some(1));
    g.untap(a.source);
    g.add_mana(a.controller, Mana::C, 1);
    Ok(())
}
fn outer_hold_mana(
    g: &mut dyn cantrip_api::Game,
    a: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    assert_eq!(
        g.mana_ability_x(),
        Some(2),
        "the outer activation's X survives nested payment"
    );
    g.add_effect(
        a.source,
        a.source,
        EffectKind::Pump(2, 0),
        Duration::WhileSourceTapped,
    );
    g.add_mana(a.controller, Mana::C, 1);
    Ok(())
}
fn outer_restart_mana(
    g: &mut dyn cantrip_api::Game,
    a: &Asker,
    color: Option<Color>,
) -> Result<(), Suspend> {
    g.tap(a.source);
    outer_hold_mana(g, a, color)
}
static MANA_HOLDER: CardDef = CardDef {
    name: "Immediate Tapped Interval Holder",
    abilities: &[
        mana_ability(
            cost("{1}, {T}").and(CHARGE_X),
            add(Mana::C).by(outer_hold_mana),
            "{1}, {T}, Remove X charge counters: Add {C}. This gets +2/+0 for as long as it remains tapped.",
        ),
        mana_ability(
            CHARGE_X,
            add(Mana::C).by(inner_retap_mana),
            "Remove X charge counters: Untap this. Tap this. Add {C}.",
        ),
    ],
    ..HOLDER
};
static RESTART_MANA_HOLDER: CardDef = CardDef {
    name: "Immediate Restart Interval Holder",
    abilities: &[
        mana_ability(
            cost("{1}, {T}").and(CHARGE_X),
            add(Mana::C).by(outer_restart_mana),
            "{1}, {T}, Remove X charge counters: Tap this. Add {C}. This gets +2/+0 for as long as it remains tapped.",
        ),
        mana_ability(
            CHARGE_X,
            add(Mana::C).by(inner_untap_mana),
            "Remove X charge counters: Untap this. Add {C}.",
        ),
    ],
    ..HOLDER
};
fn mana_position(
    g: &mut Game,
    def: &'static CardDef,
    answers: Vec<Answer>,
) -> (ObjectId, ObjectId, crate::ManaInterval) {
    let source = put(g, def, YOU, false);
    let outer = put(g, &HOLDER, YOU, true);
    g.state.update_turn(|t| t.number = 1);
    g.state.update_player(YOU, |p| p.turn_began = 1);
    crate::counters::put(
        &mut cx(g),
        source,
        CounterKind::Charge,
        3,
        YOU,
        crate::counters::How::Effect,
    );
    let interval = crate::ManaInterval {
        source: outer,
        since: 0,
        tapped_since: 0,
        resolution: 0,
        cost_objects: vec![(4, CostPart::Permanents, outer)],
    };
    g.mana_activation_x = Some(37);
    g.mana_interval = Some(interval.clone());
    let mut frame = Frame::new(FrameKind::Act { player: YOU });
    frame.answers = answers.into_iter().map(|a| (Some(YOU), a)).collect();
    g.state.frame_push(frame);
    (source, outer, interval)
}
#[test]
fn an_immediate_mana_ability_keeps_its_own_interval_through_real_nested_cost_payment() {
    for restart in [false, true] {
        let mut g = game();
        let def = if restart {
            &RESTART_MANA_HOLDER
        } else {
            &MANA_HOLDER
        };
        let (source, outer, enclosing_interval) = mana_position(&mut g, def, vec![]);
        g.state.update_frame(0, |f| {
            let mut answers = vec![
                (Some(YOU), Answer::Announce(2)),
                (
                    Some(YOU),
                    Answer::Act(Action::of(&ActionOption::on(
                        ActionKind::ActivateAbility,
                        source,
                        1,
                    ))),
                ),
                (Some(YOU), Answer::Announce(1)),
                (
                    Some(YOU),
                    Answer::Act(Action::of(&ActionOption::simple(
                        ActionKind::ConfirmCasting,
                    ))),
                ),
            ];
            // Mana abilities are activated before the outer {T} is paid. If the inner ability leaves the source
            // untapped, tap it and then spend mana; if it leaves the source tapped, try the other costs first.
            answers.push((Some(YOU), Answer::CostPayment(1)));
            if restart { answers.push((Some(YOU), Answer::CostPayment(1))); }
            f.answers = answers;
        });
        let mut cx = cx(&mut g);
        cx.frame = 0;
        cx.mana_source = Some(outer);
        let result = crate::actions::activate_mana(&mut cx, YOU, source, 0, None, false, &[]);
        if !restart {
            assert!(matches!(result, Err(crate::Stop::Cancel)), "the inner retap leaves the outer tap cost unpayable");
            assert_eq!(cx.mana_source, Some(outer));
            assert_eq!(cx.g.mana_activation_x, Some(37));
            assert_eq!(cx.g.mana_interval, Some(enclosing_interval));
            continue;
        }
        result.expect("the untap leaves the outer tap cost payable");
        assert_eq!(
            cx.s().obj(source).counters(CounterKind::Charge),
            0,
            "both independent counter costs were paid"
        );
        assert_eq!(
            cx.s().player(YOU).pool.get(Mana::C),
            1,
            "the inner mana paid the outer activation's mana cost"
        );
        assert!(cx.s().obj(source).tapped);
        assert_eq!(
            cx.chars(source).power,
            Some(4),
            "the outer interval starts at its own tap after the nested mana ability"
        );
        assert_eq!(cx.mana_source, Some(outer));
        assert_eq!(cx.g.mana_activation_x, Some(37));
        assert_eq!(cx.g.mana_interval, Some(enclosing_interval));
    }
}

#[test]
fn nested_mana_suspension_and_cancellation_restore_the_enclosing_interval_and_x() {
    for cancel in [false, true] {
        let mut g = game();
        let (source, outer, enclosing_interval) = mana_position(&mut g, &MANA_HOLDER, vec![]);
        let mut answers = vec![
            (Some(YOU), Answer::Announce(2)),
            (
                Some(YOU),
                Answer::Act(Action::of(&ActionOption::on(
                    ActionKind::ActivateAbility,
                    source,
                    1,
                ))),
            ),
        ];
        if cancel {
            answers.push((Some(YOU), Answer::Cancel));
        }
        g.state.update_frame(0, |f| f.answers = answers);
        let mut cx = cx(&mut g);
        cx.frame = 0;
        cx.mana_source = Some(outer);
        let result = crate::actions::activate_mana(&mut cx, YOU, source, 0, None, false, &[]);
        assert_eq!(
            result,
            Err(if cancel {
                crate::Stop::Cancel
            } else {
                crate::Stop::Suspend
            })
        );
        assert_eq!(cx.mana_source, Some(outer));
        assert_eq!(cx.g.mana_activation_x, Some(37));
        assert_eq!(cx.g.mana_interval, Some(enclosing_interval));
        assert_eq!(
            cx.s().obj(source).counters(CounterKind::Charge),
            3,
            "the nested announcement has not paid its counters"
        );
    }
}
