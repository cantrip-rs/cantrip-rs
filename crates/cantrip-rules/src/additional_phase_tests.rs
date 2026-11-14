//! Deterministic complete games for CR 500.8, 500.10 and 505.1a-b: ordered opening hands, explicit answers,
//! immediate timing assertions and exact event sequences; no seeds searched and no simulated phase changes.
use crate::{Game, Setup};
use cantrip_api::*;

const P: PlayerId = PlayerId(0);

fn gain(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}
fn upkeep(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.add_phases(
        PhaseAnchor::Current,
        &[PhaseSpec::Only(Step::Upkeep), PhaseSpec::Only(Step::Upkeep)],
    );
    Ok(())
}
fn beginning(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Whole(Phase::Beginning)]);
    Ok(())
}
fn mixed(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.add_phases(
        PhaseAnchor::Current,
        &[
            PhaseSpec::Whole(Phase::Combat),
            PhaseSpec::Whole(Phase::Main),
        ],
    );
    g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Only(Step::Upkeep)]);
    Ok(())
}
fn second_main(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let ids = g.add_phases(
        PhaseAnchor::Main(2),
        &[
            PhaseSpec::Whole(Phase::Combat),
            PhaseSpec::Whole(Phase::Main),
        ],
    );
    if let Some(&phase) = ids.first() {
        g.delayed_trigger(
            DelayedWhen::PhaseStep {
                step: Step::BeginCombat,
                phase,
            },
            item.object,
            item.controller,
            AbilityRef::delayed(item.ability.card, 0),
            Target::None,
        );
    }
    Ok(())
}
fn nested(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Only(Step::Draw)]);
    Ok(())
}
fn end(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.end_the_turn()
}

static FILLER: CardDef = CardDef {
    name: "Phase Filler",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};
static RELIC: CardDef = CardDef {
    name: "Phase Relic",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        triggered(
            when(EventMask::STEP_BEGAN, |_, _, e| {
                if *e == (Event::StepBegan { step: Step::Upkeep }) {
                    Fired::PLAIN
                } else {
                    None
                }
            }),
            gain,
            "Each upkeep gain 1.",
        ),
    ],
    ..CardDef::BLANK
};
static UPKEEP: CardDef = CardDef {
    name: "Two Upkeeps",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], upkeep),
    ..CardDef::BLANK
};
static BEGINNING: CardDef = CardDef {
    name: "Another Beginning",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], beginning),
    ..CardDef::BLANK
};
static MIXED: CardDef = CardDef {
    name: "Mixed Phases",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], mixed),
    ..CardDef::BLANK
};
static SECOND: CardDef = CardDef {
    name: "After Second Main",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], second_main),
    delayed: &[delayed_trigger(
        gain,
        "At the beginning of that combat gain 1.",
    )],
    ..CardDef::BLANK
};
static NESTED: CardDef = CardDef {
    name: "Nested Draw",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nested),
    ..CardDef::BLANK
};
static END: CardDef = CardDef {
    name: "End Turn",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], end),
    ..CardDef::BLANK
};

fn game(def: &'static CardDef, players: usize) -> Game {
    let table = Box::leak(Box::new(CardTable::new([
        &FILLER, &RELIC, &UPKEEP, &BEGINNING, &MIXED, &SECOND, &NESTED, &END,
    ])));
    let mut deck = vec![
        table.id_of(&RELIC),
        table.id_of(def),
        table.id_of(&NESTED),
        table.id_of(&END),
    ];
    deck.extend(vec![table.id_of(&FILLER); 30]);
    let mut decks = vec![deck];
    decks.extend((1..players).map(|_| vec![table.id_of(&FILLER); 34]));
    Game::new(
        table,
        &Setup {
            decks,
            ..Default::default()
        },
    )
}

/// Play an ordered hand, tapping the relic before the phase spell. Optional callbacks run at every real question.
fn play(
    g: &mut Game,
    def: &'static CardDef,
    mut inspect: impl FnMut(&mut Game, &Request) -> Option<Answer>,
) {
    let mut tapped = false;
    let mut cast = false;
    let mut relic = false;
    for _ in 0..10_000 {
        g.run();
        let r = g.pending().cloned().expect("question");
        if g.state.turn.number > 1 {
            return;
        }
        let answer = if let Some(a) = inspect(g, &r) {
            a
        } else {
            match &r.kind {
                RequestKind::Priority { options }
                    if r.player == P
                        && g.state.turn.step() == Step::PrecombatMain
                        && g.state.stack.is_empty() =>
                {
                    let wanted = if !relic {
                        Some((&RELIC, ActionKind::CastSpell))
                    } else if !tapped {
                        Some((&RELIC, ActionKind::ActivateAbility))
                    } else if !cast {
                        Some((def, ActionKind::CastSpell))
                    } else {
                        None
                    };
                    let o = wanted.and_then(|(d, k)| {
                        options.iter().find(|o| {
                            o.kind == k
                                && o.source
                                    .is_some_and(|id| g.state.obj(id).card == g.cards.id_of(d))
                        })
                    });
                    if let Some(o) = o {
                        if !relic {
                            relic = true;
                        } else if !tapped {
                            tapped = true;
                        } else {
                            cast = true;
                        }
                        Answer::Act(Action::of(o))
                    } else {
                        Answer::Pass
                    }
                }
                RequestKind::Priority { .. } => Answer::Pass,
                RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
                RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
                RequestKind::DiscardToHandSize { count, hand } => {
                    Answer::Cards(hand.iter().copied().take(*count as usize).collect())
                }
                other => panic!("unexpected {other:?}"),
            }
        };
        g.answer(r.id, r.player, answer).unwrap();
    }
    panic!("turn never ended");
}
fn steps(g: &Game) -> Vec<Step> {
    g.log
        .iter()
        .filter(|r| r.turn == 1)
        .filter_map(|r| {
            if let Event::StepBegan { step } = r.event {
                Some(step)
            } else {
                None
            }
        })
        .collect()
}
fn relic(g: &Game) -> ObjectId {
    *g.state
        .battlefield()
        .iter()
        .find(|&&id| g.state.obj(id).card == g.cards.id_of(&RELIC))
        .unwrap()
}

#[test]
fn upkeep_only_phases_skip_untap_and_draw_trigger_each_time_and_continue_the_same_turn() {
    let mut g = game(&UPKEEP, 3);
    let mut seen = Vec::new();
    play(&mut g, &UPKEEP, |g, r| {
        if matches!(r.kind, RequestKind::Priority { .. })
            && g.state.turn.phase_additional
            && g.state.turn.step() == Step::Upkeep
            && r.player == P
            && g.state.stack.is_empty()
        {
            assert!(
                g.state.obj(relic(g)).tapped,
                "additional upkeep must not untap"
            );
            assert!(
                g.state.player(P).pool.is_empty(),
                "main-phase mana must empty before upkeep"
            );
            assert_eq!(
                g.log
                    .iter()
                    .filter(|r| r.turn == 1 && matches!(r.event, Event::Drew { .. }))
                    .count(),
                1,
                "no additional draw"
            );
            seen.push(g.state.player(P).life);
        }
        None
    });
    assert_eq!(seen, [21, 22]);
    assert_eq!(g.state.turn.active, PlayerId(1));
    assert_eq!(
        g.history()
            .select(EventQuery::kind(EventMask::TURN_BEGAN))
            .count(),
        2,
        "one turn per player"
    );
    assert_eq!(
        steps(&g),
        [
            Step::Upkeep,
            Step::Draw,
            Step::PrecombatMain,
            Step::Upkeep,
            Step::Upkeep,
            Step::BeginCombat,
            Step::DeclareAttackers,
            Step::EndOfCombat,
            Step::PostcombatMain,
            Step::End,
            Step::Cleanup
        ]
    );
}

#[test]
fn a_whole_beginning_phase_untaps_draws_and_does_not_begin_a_new_turn() {
    let mut g = game(&BEGINNING, 2);
    let mut seen = false;
    play(&mut g, &BEGINNING, |g, r| {
        if r.player == P
            && g.state.turn.phase_additional
            && g.state.turn.step() == Step::Draw
            && g.state.stack.is_empty()
        {
            assert!(!g.state.obj(relic(g)).tapped);
            assert_eq!(g.state.player(P).life, 21);
            assert_eq!(
                g.log
                    .iter()
                    .filter(|r| r.turn == 1 && matches!(r.event, Event::Drew { .. }))
                    .count(),
                1,
                "additional draw is not the skipped ordinary draw"
            );
            assert_eq!(
                g.history()
                    .select(EventQuery::kind(EventMask::TURN_BEGAN))
                    .count(),
                1
            );
            seen = true;
        }
        None
    });
    assert!(seen);
}

#[test]
fn later_groups_go_first_and_combat_then_main_keeps_its_printed_order() {
    let mut g = game(&MIXED, 2);
    play(&mut g, &MIXED, |_, _| None);
    assert_eq!(
        steps(&g),
        [
            Step::Upkeep,
            Step::PrecombatMain,
            Step::Upkeep,
            Step::BeginCombat,
            Step::DeclareAttackers,
            Step::EndOfCombat,
            Step::PostcombatMain,
            Step::BeginCombat,
            Step::DeclareAttackers,
            Step::EndOfCombat,
            Step::PostcombatMain,
            Step::End,
            Step::Cleanup
        ]
    );
}

#[test]
fn second_main_anchor_and_that_combat_trigger_ignore_the_ordinary_combat() {
    let mut g = game(&SECOND, 2);
    let mut ordinary = false;
    let mut additional = false;
    play(&mut g, &SECOND, |g, r| {
        if r.player == P && g.state.turn.step() == Step::BeginCombat && g.state.stack.is_empty() {
            if g.state.turn.phase_additional {
                assert_eq!(g.state.player(P).life, 21);
                additional = true;
            } else {
                assert_eq!(g.state.player(P).life, 20);
                ordinary = true;
            }
        }
        None
    });
    assert!(ordinary && additional);
    assert!(g.state.delayed.is_empty());
    assert_eq!(
        steps(&g),
        [
            Step::Upkeep,
            Step::PrecombatMain,
            Step::BeginCombat,
            Step::DeclareAttackers,
            Step::EndOfCombat,
            Step::PostcombatMain,
            Step::BeginCombat,
            Step::DeclareAttackers,
            Step::EndOfCombat,
            Step::PostcombatMain,
            Step::End,
            Step::Cleanup
        ]
    );
}

#[test]
fn insertions_inside_an_added_main_resume_the_pending_ordinary_combat() {
    let mut g = game(&MIXED, 2);
    let mut cast = false;
    play(&mut g, &MIXED, |g, r| {
        if !cast
            && g.state.turn.phase_additional
            && g.state.turn.step().is_main()
            && g.state.stack.is_empty()
        {
            if let RequestKind::Priority { options } = &r.kind {
                if let Some(o) = options.iter().find(|o| {
                    o.kind == ActionKind::CastSpell
                        && o.source
                            .is_some_and(|id| g.state.obj(id).card == g.cards.id_of(&NESTED))
                }) {
                    cast = true;
                    return Some(Answer::Act(Action::of(o)));
                }
            }
        }
        None
    });
    assert!(cast);
    let events = steps(&g);
    let added_main = events
        .iter()
        .position(|s| *s == Step::PostcombatMain)
        .unwrap();
    assert_eq!(
        &events[added_main..added_main + 3],
        &[Step::PostcombatMain, Step::Draw, Step::BeginCombat]
    );
}

#[test]
fn ending_the_turn_discards_added_phases_and_their_delayed_trigger() {
    let mut g = game(&SECOND, 2);
    let mut cast = false;
    play(&mut g, &SECOND, |g, r| {
        if !cast
            && g.state.turn.scheduled
            && g.state.turn.step() == Step::BeginCombat
            && g.state.stack.is_empty()
        {
            if let RequestKind::Priority { options } = &r.kind {
                if let Some(o) = options.iter().find(|o| {
                    o.kind == ActionKind::CastSpell
                        && o.source
                            .is_some_and(|id| g.state.obj(id).card == g.cards.id_of(&END))
                }) {
                    cast = true;
                    return Some(Answer::Act(Action::of(o)));
                }
            }
        }
        None
    });
    assert!(cast);
    assert!(g.state.phase_schedule.is_empty());
    assert!(!g.state.turn.scheduled);
    assert!(g.state.delayed.is_empty());
    assert_eq!(g.state.player(P).life, 20);
    assert!(!steps(&g).contains(&Step::PostcombatMain));
}

#[test]
fn schedule_identity_and_remaining_phases_survive_game_tree_undo_and_redo() {
    let mut g = game(&MIXED, 2);
    let mut checked = false;
    play(&mut g, &MIXED, |g, r| {
        if !checked && g.state.turn.scheduled && matches!(r.kind, RequestKind::Priority { .. }) {
            let node = g.at;
            let old = g.tree.parent(node).expect("parent");
            let schedule = g.state.phase_schedule.clone();
            let turn = g.state.turn;
            g.goto(old);
            g.goto(node);
            assert_eq!(g.state.phase_schedule, schedule);
            assert_eq!(g.state.turn, turn);
            checked = true;
        }
        None
    });
    assert!(checked);
}
