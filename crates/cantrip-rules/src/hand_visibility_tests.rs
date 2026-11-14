use crate::{Cx, Game, Setup, hand_visibility, ops};
use cantrip_api::{Game as Rules, *};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn all(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}
fn you(_: &dyn Query, a: &Asker, p: PlayerId) -> bool {
    p == a.controller
}
fn tapped(q: &dyn Query, a: &Asker) -> bool {
    q.state().obj(a.source).tapped
}
static OPEN: CardDef = CardDef {
    name: "Public hand fixture",
    play_rules: &[PlayRule::HandRevealed(all)],
    ..CardDef::BLANK
};
static OWN: CardDef = CardDef {
    name: "Own public hand fixture",
    play_rules: &[PlayRule::HandRevealed(you)],
    ..OPEN
};
static CONDITIONAL: CardDef = CardDef {
    name: "Conditional public hand fixture",
    play_rules: &[PlayRule::While(tapped, &[PlayRule::HandRevealed(all)])],
    ..OPEN
};
static GIVEN: CardDef = CardDef {
    name: "Granted public hand fixture",
    play_rules: &[],
    delayed: &[granted_play_rule(
        PlayRule::HandRevealed(you),
        "Your hand is revealed.",
    )],
    ..OPEN
};
static FILL: CardDef = CardDef {
    name: "Visibility filler",
    ..CardDef::BLANK
};

fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([
        &OPEN,
        &OWN,
        &CONDITIONAL,
        &GIVEN,
        &FILL,
    ])));
    Game::new(
        table,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    )
}
fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![],
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn sync(g: &mut Game) {
    hand_visibility::sync(&mut cx(g));
}

#[test]
fn visibility_follows_the_source_controller_and_overlapping_rules() {
    let mut g = game();
    let own = put(&mut g, &OWN, A, Zone::BATTLEFIELD);
    assert_eq!(g.revealed_hands(), 1);
    g.state.update_obj(own, |o| o.controller = B);
    assert_eq!(g.revealed_hands(), 2);
    let open = put(&mut g, &OPEN, A, Zone::BATTLEFIELD);
    assert_eq!(g.revealed_hands(), 3);
    cx(&mut g).add_effect(
        open,
        open,
        EffectKind::LoseAllAbilities,
        Duration::Indefinite,
    );
    assert_eq!(g.revealed_hands(), 2);
}

#[test]
fn conditions_and_phasing_stop_public_visibility() {
    let mut g = game();
    let source = put(&mut g, &CONDITIONAL, A, Zone::BATTLEFIELD);
    assert_eq!(g.revealed_hands(), 0);
    cx(&mut g).tap(source);
    assert_eq!(g.revealed_hands(), 3);
    g.state.zone_remove(Zone::BATTLEFIELD, source);
    g.state.update_obj(source, |o| o.zone = Zone::PHASED_OUT);
    g.state.zone_push(Zone::PHASED_OUT, source);
    assert_eq!(g.revealed_hands(), 0);
}

#[test]
fn granted_rules_use_the_holders_controller_and_obey_later_ability_loss() {
    let mut g = game();
    let holder = put(&mut g, &GIVEN, B, Zone::BATTLEFIELD);
    let a = AbilityRef::delayed(g.cards.id_of(&GIVEN), 0);
    assert_eq!(g.revealed_hands(), 0);
    cx(&mut g).add_effect(
        holder,
        holder,
        EffectKind::AddAbility(a),
        Duration::Indefinite,
    );
    assert_eq!(g.revealed_hands(), 2);
    cx(&mut g).add_effect(
        holder,
        holder,
        EffectKind::LoseAllAbilities,
        Duration::Indefinite,
    );
    assert_eq!(g.revealed_hands(), 0);
}

#[test]
fn showing_an_existing_hand_is_once_per_interval_and_rolls_back() {
    let mut g = game();
    let card = put(&mut g, &FILL, B, Zone::hand(B));
    let source = put(&mut g, &CONDITIONAL, A, Zone::BATTLEFIELD);
    let mark = g.state.mark();
    let log = g.log.len();
    cx(&mut g).tap(source);
    sync(&mut g);
    sync(&mut g);
    assert_eq!(
        g.log
            .iter()
            .filter(|r| matches!(r.event, Event::Shown { player:A,card:c } if c==card))
            .count(),
        1
    );
    assert!(
        !g.log
            .iter()
            .any(|r| matches!(r.event, Event::Revealed { .. }))
    );
    g.state.revert(mark);
    g.log.truncate(log);
    assert_eq!(g.state.shown_hands, 0);
    cx(&mut g).tap(source);
    assert_eq!(
        g.log
            .iter()
            .filter(|r| matches!(r.event, Event::Shown { player:A,card:c } if c==card))
            .count(),
        1
    );
}

#[test]
fn a_revealed_hand_move_records_the_old_visibility_and_new_identity() {
    let mut g = game();
    put(&mut g, &OPEN, A, Zone::BATTLEFIELD);
    let card = put(&mut g, &FILL, B, Zone::hand(B));
    let moved = ops::move_together(
        &mut cx(&mut g),
        &[ops::Moving::to(card, Zone::library(B))],
        ZoneReason::Other,
    );
    assert!(g.log.iter().any(|r| matches!(r.event, Event::ZoneChange { old,new,from_revealed_hand:true,to_revealed_hand:false,.. } if old==card && new==moved[0])));
}

#[test]
fn a_simultaneous_departure_does_not_show_the_card_arriving_in_the_hand() {
    let mut g = game();
    let source = put(&mut g, &OPEN, A, Zone::BATTLEFIELD);
    let card = put(&mut g, &FILL, B, Zone::library(B));
    ops::move_together(
        &mut cx(&mut g),
        &[
            ops::Moving::to(source, Zone::graveyard(A)),
            ops::Moving::to(card, Zone::hand(B)),
        ],
        ZoneReason::Other,
    );
    assert_eq!(g.revealed_hands(), 0);
    assert!(g.log.iter().any(|r|matches!(r.event, Event::ZoneChange {old,from_revealed_hand:false,to_revealed_hand:false,..} if old==card)));
}

#[test]
fn a_simultaneous_arrival_opens_the_hand_after_the_move() {
    let mut g = game();
    let source = put(&mut g, &OPEN, A, Zone::hand(A));
    let card = put(&mut g, &FILL, B, Zone::library(B));
    ops::move_together(
        &mut cx(&mut g),
        &[
            ops::Moving::to(source, Zone::BATTLEFIELD),
            ops::Moving::to(card, Zone::hand(B)),
        ],
        ZoneReason::Other,
    );
    assert_eq!(g.revealed_hands(), 3);
    assert!(g.log.iter().any(|r|matches!(r.event, Event::ZoneChange {old,from_revealed_hand:false,to_revealed_hand:true,..} if old==card)));
}

#[test]
fn merely_cataloging_revealers_does_not_open_a_hand() {
    let mut g = game();
    put(&mut g, &OPEN, A, Zone::hand(A));
    put(&mut g, &FILL, B, Zone::hand(B));
    sync(&mut g);
    assert_eq!(g.revealed_hands(), 0);
    assert!(g.log.is_empty());
}

#[test]
fn the_interest_cache_survives_temporary_queries_without_reusing_their_position() {
    let mut g = game();
    assert_eq!(g.revealed_hands(), 0);
    let mark = g.state.mark();
    let aside = g.state.set_derived_aside();
    put(&mut g, &OPEN, A, Zone::BATTLEFIELD);
    assert_eq!(g.revealed_hands(), 3);
    g.state.revert(mark);
    g.state.restore_derived(aside);
    assert_eq!(g.revealed_hands(), 0);
    put(&mut g, &FILL, A, Zone::BATTLEFIELD);
    assert_eq!(g.revealed_hands(), 0);
    put(&mut g, &OPEN, A, Zone::BATTLEFIELD);
    assert_eq!(g.revealed_hands(), 3);
}
