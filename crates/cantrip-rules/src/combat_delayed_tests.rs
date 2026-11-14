//! Event watchers whose stated duration is this combat (Goblin Flotilla, Melee).
use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}
fn tapped(_: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    if let Event::Tapped { permanent } = event {
        Fired::about(Target::Object(*permanent))
    } else {
        None
    }
}
fn control(_: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    if let Event::ControlChanged { permanent, .. } = event {
        Fired::about(Target::Object(*permanent))
    } else {
        None
    }
}
static SOURCE: CardDef = CardDef {
    name: "Combat duration event watcher",
    types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[
        triggered(
            when_asked(EventMask::TAPPED, tapped),
            nothing,
            "Whenever a permanent becomes tapped this combat.",
        ),
        triggered(
            when_asked(EventMask::CONTROL_CHANGED, control),
            nothing,
            "Whenever control changes this combat.",
        ),
    ],
    ..CardDef::BLANK
};
static BODY: CardDef = CardDef {
    name: "Combat duration body",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: 0,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: Vec::new(),
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn board() -> (Game, ObjectId, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &BODY])));
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![vec![cards.id_of(&BODY); 20]; 2],
            shuffle: false,
            ..Default::default()
        },
    );
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::Settle));
    g.state.update_turn(|t| {
        t.number = 4;
        t.active = A;
        t.priority = A;
        t.began = true;
        t.set_step(Step::DeclareAttackers);
    });
    g.state.update_player(A, |p| p.turn_began = 4);
    let mut put = |def: &'static CardDef, zone: Zone| {
        let id = g.state.push_obj(Obj::new(cards.id_of(def), A, zone));
        g.state.zone_push(zone, id);
        id
    };
    let source = put(&SOURCE, Zone::BATTLEFIELD);
    let body = put(&BODY, Zone::BATTLEFIELD);
    g.state.see_statics();
    (g, source, body)
}
fn watch(g: &mut Game, source: ObjectId, index: u8, when: DelayedWhen) {
    let ability = AbilityRef::delayed(g.state.obj(source).card, index);
    cx(g).delayed_trigger(when, source, A, ability, Target::None);
}
fn tap(g: &mut Game, body: ObjectId) {
    cx(g).tap(body);
}
#[test]
fn repeats_through_end_combat_priority_then_expires_without_cancelling_pending_triggers() {
    let (mut g, s, b) = board();
    watch(&mut g, s, 0, DelayedWhen::ThisCombat);
    tap(&mut g, b);
    cx(&mut g).untap(b);
    tap(&mut g, b);
    assert_eq!(g.state.triggers.len(), 2);
    assert_eq!(g.state.delayed.len(), 1);
    g.state.update_turn(|t| t.set_step(Step::EndOfCombat));
    cx(&mut g).untap(b);
    tap(&mut g, b);
    assert_eq!(g.state.triggers.len(), 3);
    crate::turn::next_step(&mut g);
    assert_eq!(g.state.turn.step(), Step::PostcombatMain);
    assert!(g.state.delayed.is_empty());
    assert_eq!(g.state.triggers.len(), 3);
    cx(&mut g).untap(b);
    tap(&mut g, b);
    assert_eq!(g.state.triggers.len(), 3);
}
#[test]
fn an_additional_combat_does_not_inherit_the_previous_combats_watchers() {
    let (mut g, s, b) = board();
    watch(&mut g, s, 0, DelayedWhen::ThisCombat);
    g.state.update_turn(|t| t.set_step(Step::EndOfCombat));
    cx(&mut g).add_combat_phase();
    crate::turn::next_step(&mut g);
    assert_eq!(g.state.turn.step(), Step::BeginCombat);
    assert!(g.state.delayed.is_empty());
    tap(&mut g, b);
    assert!(g.state.triggers.is_empty());
}
#[test]
fn ending_combat_or_turn_early_expires_the_combat_watcher_but_preserves_turn_watchers() {
    for ends_turn in [false, true] {
        let (mut g, s, b) = board();
        watch(&mut g, s, 0, DelayedWhen::ThisCombat);
        watch(&mut g, s, 0, DelayedWhen::ThisTurn);
        if ends_turn {
            crate::turn::end_the_turn(&mut cx(&mut g)).unwrap();
        } else {
            crate::turn::end_the_combat_phase(&mut cx(&mut g)).unwrap();
        }
        crate::turn::next_step(&mut g);
        assert_eq!(
            g.state.turn.step(),
            if ends_turn {
                Step::Cleanup
            } else {
                Step::PostcombatMain
            }
        );
        assert_eq!(g.state.delayed.len(), 1);
        assert_eq!(g.state.delayed[0].when, DelayedWhen::ThisTurn);
        tap(&mut g, b);
        assert_eq!(g.state.triggers.len(), 1);
    }
}
#[test]
fn losing_the_creators_abilities_or_leaving_does_not_remove_a_created_watcher() {
    let (mut g, s, b) = board();
    watch(&mut g, s, 0, DelayedWhen::ThisCombat);
    cx(&mut g).add_effect(s, s, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    tap(&mut g, b);
    assert_eq!(g.state.triggers.len(), 1);
    cx(&mut g).return_to_hand(&[s]);
    cx(&mut g).untap(b);
    tap(&mut g, b);
    assert_eq!(g.state.triggers.len(), 2);
    assert!(
        g.state
            .triggers
            .iter()
            .all(|t| t.source == s && t.controller == A)
    );
}
#[test]
fn a_watcher_created_outside_combat_ends_at_cleanup_without_waiting_for_another_turn() {
    let (mut g, s, _) = board();
    g.state.update_turn(|t| t.set_step(Step::PostcombatMain));
    watch(&mut g, s, 0, DelayedWhen::ThisCombat);
    g.state.update_turn(|t| t.set_step(Step::Cleanup));
    crate::turn::cleanup(&mut cx(&mut g)).unwrap();
    assert!(g.state.delayed.is_empty());
}
#[test]
fn expiry_is_journalled_and_undo_restores_the_same_watcher() {
    let (mut g, s, _) = board();
    watch(&mut g, s, 0, DelayedWhen::ThisCombat);
    g.state.update_turn(|t| t.set_step(Step::EndOfCombat));
    let d = g.state.delayed[0];
    let mark = g.state.mark();
    crate::turn::next_step(&mut g);
    assert!(g.state.delayed.is_empty());
    g.state.rollback(mark);
    assert_eq!(g.state.delayed.as_slice(), &[d]);
    assert_eq!(g.state.turn.step(), Step::EndOfCombat);
}
#[test]
fn a_control_effect_ending_at_the_same_boundary_can_still_trigger_the_watcher() {
    let (mut g, s, b) = board();
    cx(&mut g).add_effect(s, b, EffectKind::Control(B), Duration::EndOfCombat);
    assert_eq!(g.state.obj(b).controller, B);
    watch(&mut g, s, 1, DelayedWhen::ThisCombat);
    g.state.update_turn(|t| t.set_step(Step::EndOfCombat));
    crate::turn::next_step(&mut g);
    assert_eq!(g.state.obj(b).controller, A);
    assert!(g.state.delayed.is_empty());
    assert_eq!(g.state.triggers.len(), 1);
    assert_eq!(g.state.triggers[0].subject, Target::Object(b));
}
