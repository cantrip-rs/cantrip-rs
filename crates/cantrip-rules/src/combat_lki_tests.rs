//! Source combat relationships survive leaving through the same LKI authority
//! as characteristics (CR 608.2h, 113.7a), captured before simultaneous removal.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Attacker, Block, Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static CREATURE: CardDef = CardDef {
    name: "Combat relationship creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};
fn tinder_target(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    target.object().is_some_and(|id| {
        q.is_on_battlefield(id)
            && q.chars(id).is(CardType::Creature)
            && q.creatures_blocked_by(asker.source).contains(&id)
    })
}
fn tinder_damage(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0) {
        g.damage(item.object, g.state().target(item, 0), 2);
    }
    Ok(())
}
/// Tinder Wall's actual damage cost and target restriction (CR 602.2b, 608.2b,h).
static TINDER: CardDef = CardDef {
    name: "Combat LKI Tinder Wall",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Defender]),
    abilities: &[ability(
        cost("{R}").and(SACRIFICE_THIS),
        tinder_damage,
        "{R}, Sacrifice this creature: It deals 2 damage to target creature it's blocking.",
    )
    .targets(&[target(tinder_target)])],
    ..CardDef::BLANK
};
fn position() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&CREATURE, &TINDER])));
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![vec![cards.id_of(&CREATURE); 20]; 2],
            ..Default::default()
        },
    );
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = THEM;
        t.priority = YOU;
        t.set_step(Step::DeclareBlockers);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}
fn put(g: &mut Game, owner: PlayerId) -> ObjectId {
    put_def(g, &CREATURE, owner)
}
fn put_def(g: &mut Game, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = g
        .state
        .push_obj(Obj::new(g.cards.id_of(def), owner, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
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
fn pair(g: &mut Game, blocker: ObjectId, attacker: ObjectId) {
    if !g.state.attackers.iter().any(|a| a.id == attacker) {
        let mut attackers = g.state.attackers.clone();
        let mut a = Attacker::new(attacker, Target::Player(YOU), YOU, TypeSet::NONE);
        a.blocked = true;
        attackers.push(a);
        g.state.set_attackers(attackers);
    }
    let mut blocks = g.state.blocks.clone();
    blocks.push(Block {
        blocker,
        attacker,
        first_strike: false,
    });
    g.state.set_blocks(blocks);
}

fn blocked_by(q: &dyn Query, source: ObjectId) -> Vec<ObjectId> {
    q.creatures_blocked_by(source)
}
fn blocking(q: &dyn Query, source: ObjectId) -> Vec<ObjectId> {
    q.creatures_blocking(source)
}

#[test]
fn an_attacker_keeps_its_last_known_blockers_when_it_dies() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    let c = put(&mut g, YOU);
    pair(&mut g, b, a);
    pair(&mut g, c, a);
    crate::ops::sacrifice(&mut cx(&mut g), THEM, &[a]);
    assert!(g.state.blocks.is_empty());
    assert_eq!(
        blocking(&g, a),
        [b, c],
        "Abu Ja'far's death ability asks its last-known blockers"
    );
}

#[test]
fn a_sacrificed_blocker_keeps_the_creatures_it_was_blocking() {
    let mut g = position();
    let wall = put(&mut g, YOU);
    let a = put(&mut g, THEM);
    let b = put(&mut g, THEM);
    pair(&mut g, wall, a);
    pair(&mut g, wall, b);
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[wall]);
    assert_eq!(
        blocked_by(&g, wall),
        [a, b],
        "Tinder Wall's target filter still reads the sacrificed source"
    );
}

#[test]
fn simultaneous_deaths_capture_the_whole_pre_move_graph_in_both_directions() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    let c = put(&mut g, YOU);
    pair(&mut g, b, a);
    pair(&mut g, c, a);
    cantrip_api::Game::destroy_all_no_regen(&mut cx(&mut g), &[b, a]);
    assert_eq!(blocking(&g, a), [b, c]);
    assert_eq!(blocked_by(&g, b), [a]);
    assert!(
        blocked_by(&g, c).is_empty(),
        "the live blocker no longer blocks the departed attacker"
    );
}

#[test]
fn an_earlier_combat_removal_is_not_revived_by_a_later_death() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    pair(&mut g, b, a);
    crate::ops::remove_from_combat(&mut cx(&mut g), &[b]);
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[b]);
    assert!(blocked_by(&g, b).is_empty());
}

#[test]
fn a_live_sources_relationships_are_always_current() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    pair(&mut g, b, a);
    assert_eq!(blocked_by(&g, b), [a]);
    assert_eq!(blocking(&g, a), [b]);
    crate::ops::remove_from_combat(&mut cx(&mut g), &[a]);
    assert!(blocked_by(&g, b).is_empty());
    assert!(blocking(&g, a).is_empty());
    assert!(
        g.is_blocking(b),
        "the independent blocking status remains true after its attacker leaves combat"
    );
}

#[test]
fn cloned_positions_keep_the_snapshot_and_rollback_truncates_its_arena() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    pair(&mut g, b, a);
    let mark = g.state.mark();
    let before = g.state.members.len();
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[b]);
    let cloned = g.clone();
    assert_eq!(
        blocked_by(&cloned, b),
        [a],
        "LKI must not depend on a clone's discarded undo history"
    );
    assert!(g.state.members.len() > before);
    g.state.rollback(mark);
    assert_eq!(g.state.members.len(), before);
    assert_eq!(blocked_by(&g, b), [a]);
    assert!(g.state.last_known(b).is_none());
    crate::ops::remove_from_combat(&mut cx(&mut g), &[b]);
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[b]);
    assert!(
        blocked_by(&g, b).is_empty(),
        "a different replay branch captures its own graph"
    );
    assert_eq!(
        blocked_by(&cloned, b),
        [a],
        "the earlier clone retains the original graph"
    );
}

#[test]
fn phasing_uses_the_pre_phase_graph_and_phase_in_does_not_restore_combat() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    pair(&mut g, b, a);
    cantrip_api::Game::phase_out(&mut cx(&mut g), &[b, a]);
    assert_eq!(blocked_by(&g, b), [a]);
    assert_eq!(blocking(&g, a), [b]);
    assert!(!g.is_blocking(b));
    assert!(!g.is_attacking(a));
    crate::phasing::untap_step(&mut cx(&mut g), YOU);
    assert!(g.is_on_battlefield(b));
    assert!(blocked_by(&g, b).is_empty());
    assert_eq!(
        blocking(&g, a),
        [b],
        "the still-phased attacker's own snapshot is immutable"
    );
    crate::phasing::untap_step(&mut cx(&mut g), THEM);
    assert!(g.is_on_battlefield(a));
    assert!(blocking(&g, a).is_empty());
}

#[test]
fn control_change_and_losing_creature_type_remove_current_pairs_before_lki() {
    for control in [false, true] {
        let mut g = position();
        let a = put(&mut g, THEM);
        let b = put(&mut g, YOU);
        pair(&mut g, b, a);
        let kind = if control {
            EffectKind::Control(THEM)
        } else {
            EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact]))
        };
        cantrip_api::Game::add_effect(&mut cx(&mut g), b, b, kind, Duration::EndOfTurn);
        assert!(
            blocked_by(&g, b).is_empty(),
            "removed from combat while still on the battlefield"
        );
        let player = g.controller(b);
        crate::ops::sacrifice(&mut cx(&mut g), player, &[b]);
        assert!(
            blocked_by(&g, b).is_empty(),
            "a later death cannot revive a former pairing"
        );
    }
}

#[test]
fn later_combat_and_later_incarnations_do_not_replace_snapshot_members() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    let c = put(&mut g, THEM);
    pair(&mut g, b, a);
    crate::ops::remove_from_combat(&mut cx(&mut g), &[a, b]);
    pair(&mut g, b, c);
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[b]);
    assert_eq!(
        blocked_by(&g, b),
        [c],
        "the prior combat relationship is absent"
    );
    let hand = cantrip_api::Game::return_to_hand(&mut cx(&mut g), &[c])[0];
    let returned =
        cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), hand, THEM, false).unwrap();
    assert_ne!(c, returned);
    assert_eq!(
        blocked_by(&g, b),
        [c],
        "the snapshot never follows became to a new object"
    );
    assert!(!blocked_by(&g, b).contains(&returned));
    assert!(blocking(&g, returned).is_empty());
}

#[test]
fn only_reading_a_historical_relationship_notifies_the_history_watch() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put(&mut g, YOU);
    pair(&mut g, b, a);
    cantrip_api::history::take_reads();
    assert_eq!(blocked_by(&g, b), [a]);
    assert_eq!(cantrip_api::history::take_reads(), 0);
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[b]);
    cantrip_api::history::take_reads();
    assert_eq!(blocked_by(&g, b), [a]);
    assert!(cantrip_api::history::take_reads() > 0);
}

#[test]
fn arena_offsets_are_not_loop_identity_and_hidden_rebinding_preserves_public_relations() {
    let mut g = position();
    let a = put(&mut g, THEM);
    let b = put_def(&mut g, &TINDER, YOU);
    pair(&mut g, b, a);
    crate::ops::sacrifice(&mut cx(&mut g), YOU, &[b]);
    let item = crate::actions::stack_item(
        StackKind::Activated,
        b,
        AbilityRef {
            card: g.cards.id_of(&TINDER),
            index: 0,
        },
        YOU,
        &[Target::Object(a)],
    );
    g.state.stack_push(item);
    let before = crate::loops::position(&g.state);
    let last = *g.state.last_known(b).unwrap();
    let members = g.state.members(last.blocked_creatures).to_vec();
    g.state.members_push(&[ObjectId::NONE]);
    let duplicate = g.state.members_push(&members);
    assert_ne!(last.blocked_creatures.start, duplicate.start);
    let row = g.state.last_known_push(LastKnown {
        blocked_creatures: duplicate,
        ..last
    });
    g.state.update_obj(b, |o| o.last_known = Some(row));
    assert_eq!(
        crate::loops::position(&g.state),
        before,
        "history offsets are normalized just like event history"
    );
    let rebound = LastKnown {
        blocked_creatures: Default::default(),
        blocking_creatures: Default::default(),
        name: "Rebound hidden characteristics",
        ..last
    };
    g.state.rebind_hidden_rows(&[b], &[(b, rebound)], |_| true);
    assert_eq!(
        blocked_by(&g, b),
        [a],
        "private characteristic sampling does not erase public combat history"
    );
}

fn pending(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question is pending")
}
fn answer(g: &mut Game, request: &Request, answer: Answer) {
    g.answer(request.id, request.player, answer.clone())
        .unwrap_or_else(|e| panic!("{e:?}; answering {answer:?} to {request:?}"));
}
fn activated_tinder() -> (Game, ObjectId, ObjectId) {
    let mut g = position();
    let wall = put_def(&mut g, &TINDER, YOU);
    let attacker = put(&mut g, THEM);
    pair(&mut g, wall, attacker);
    g.state
        .add_mana(YOU, Mana::R, 1, wall, None, ManaLasts::Step, false);
    let request = pending(&mut g);
    let RequestKind::Priority { options } = &request.kind else {
        panic!("priority expected: {:?}", request.kind)
    };
    let action = options
        .iter()
        .find(|o| o.source == Some(wall) && o.kind == ActionKind::ActivateAbility)
        .cloned()
        .expect("Tinder's paid ability is offered");
    let mut chosen = Action::of(&action);
    chosen.targets = vec![Target::Object(attacker)];
    answer(&mut g, &request, Answer::Act(chosen));
    loop {
        let request = pending(&mut g);
        match &request.kind {
            RequestKind::ChooseTarget { .. } => answer(
                &mut g,
                &request,
                Answer::Targets(vec![Target::Object(attacker)]),
            ),
            RequestKind::PayCosts { options, .. } => {
                let confirm = options
                    .iter()
                    .find(|o| o.kind == ActionKind::ConfirmCasting)
                    .cloned()
                    .expect("one red mana pays exactly");
                answer(&mut g, &request, Answer::Act(Action::of(&confirm)));
            }
            RequestKind::ChooseCostPayment { choices, .. } => {
                let mana = choices.iter().position(|c| c.kind == CostPaymentKind::Mana).expect("the mana can be paid first");
                answer(&mut g, &request, Answer::CostPayment(mana));
            }
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected activation question {other:?}"),
        }
    }
    assert!(
        g.state.obj(wall).became.is_some(),
        "the actual sacrifice cost has been paid"
    );
    assert_eq!(
        g.state.player(YOU).pool.total(),
        0,
        "the exact red mana was paid too"
    );
    assert_eq!(g.state.stack.len(), 1);
    (g, wall, attacker)
}
fn resolve_all(g: &mut Game) {
    while !g.state.stack.is_empty() {
        let request = pending(g);
        assert!(
            matches!(request.kind, RequestKind::Priority { .. }),
            "{:?}",
            request.kind
        );
        answer(g, &request, Answer::Pass);
    }
    g.run();
}
/// The activation above is paid through actual offered actions. An intervening
/// response is then forced with a rules verb; resolve from that resulting state
/// without answering the pre-response pending question (which would rewind it).
fn resolve_forced_response(g: &mut Game) {
    let serial = g
        .state
        .stack
        .last()
        .expect("the paid ability remains on the stack")
        .serial;
    let at = g.state.agenda.len();
    let mut frame = Frame::new(FrameKind::Resolve { serial });
    frame.log_mark = g.log.len() as u32;
    g.state.frame_push(frame);
    let mut frame = cx(g);
    frame.frame = at;
    crate::resolve::resolve(&mut frame).expect("the actual target recheck and effect resolve");
    assert!(frame.s().stack.is_empty());
}

#[test]
fn tinder_walls_real_cost_does_not_make_its_target_illegal() {
    let (mut g, wall, attacker) = activated_tinder();
    assert_eq!(blocked_by(&g, wall), [attacker]);
    resolve_all(&mut g);
    assert_eq!(g.state.obj(attacker).damage, 2);
    let known = g
        .log
        .iter()
        .find_map(|record| match record.event {
            Event::Damage { source, known, .. } if source == wall => Some(known),
            _ => None,
        })
        .expect("the sacrificed source actually dealt damage");
    assert_eq!(
        g.state
            .members(g.state.last_known[known as usize].blocked_creatures),
        [attacker],
        "a later damage event captures the same departed-source information"
    );
}

#[test]
fn a_later_removal_from_combat_does_not_change_the_departed_sources_lki() {
    let (mut g, _, attacker) = activated_tinder();
    crate::ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    // CR 608.2b explicitly uses the departed ability source's last-known information for target rechecking.
    // The same target incarnation remains a creature, so changing its combat status does not change that source data.
    assert!(!g.is_attacking(attacker));
    resolve_forced_response(&mut g);
    assert!(!g.is_attacking(attacker));
    assert_eq!(g.state.obj(attacker).damage, 2);
}

#[test]
fn a_target_that_changes_zones_is_not_followed_by_tinder_walls_ability() {
    let (mut g, _, attacker) = activated_tinder();
    let hand = cantrip_api::Game::return_to_hand(&mut cx(&mut g), &[attacker])[0];
    let returned =
        cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), hand, THEM, false).unwrap();
    assert!(g.is_on_battlefield(returned));
    resolve_forced_response(&mut g);
    assert!(g.is_on_battlefield(returned));
    assert_eq!(
        g.state.obj(returned).damage,
        0,
        "CR 608.2b/400.7: this is a new target incarnation"
    );
}

#[test]
fn a_last_known_attacker_can_have_more_than_255_blockers() {
    let mut g = position();
    let attacker = put(&mut g, THEM);
    let blockers: Vec<_> = (0..512).map(|_| put(&mut g, YOU)).collect();
    for &blocker in &blockers {
        pair(&mut g, blocker, attacker);
    }
    crate::ops::sacrifice(&mut cx(&mut g), THEM, &[attacker]);
    assert_eq!(blocking(&g, attacker), blockers);
}

#[test]
fn the_game_log_tree_undoes_and_redoes_paid_source_lki_with_its_arena() {
    let (mut g, wall, attacker) = activated_tinder();
    let paid = g.at;
    resolve_all(&mut g);
    let resolved = g.at;
    assert_eq!(g.state.obj(attacker).damage, 2);
    g.goto(paid);
    assert_eq!(blocked_by(&g, wall), [attacker]);
    assert_eq!(g.state.obj(attacker).damage, 0);
    g.goto(resolved);
    assert_eq!(blocked_by(&g, wall), [attacker]);
    assert_eq!(g.state.obj(attacker).damage, 2);
}

#[test]
fn target_characteristics_are_current_even_though_the_source_has_lki() {
    let (mut g, wall, attacker) = activated_tinder();
    cantrip_api::Game::add_effect(
        &mut cx(&mut g),
        attacker,
        attacker,
        EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])),
        Duration::EndOfTurn,
    );
    assert_eq!(blocked_by(&g, wall), [attacker]);
    assert!(!g.chars(attacker).is(CardType::Creature));
    resolve_forced_response(&mut g);
    assert_eq!(
        g.state.obj(attacker).damage,
        0,
        "CR 608.2b checks the target's current type"
    );
}
