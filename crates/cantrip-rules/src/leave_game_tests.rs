//! A player leaving a game of three (CR 800.4a), over test cards: their objects leave together, each as it was before any of
//! them left; the effects that gave them control end; then what they still control is exiled.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn plus_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = c.power.map(|p| p + 1);
    c.toughness = c.toughness.map(|t| t + 1);
}

/// "Creatures get +1/+1."
static ANTHEM: CardDef = CardDef {
    name: "Leave Test Anthem",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtModify, creature, plus_one, "")],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Leave Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn enchanted(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    q.state().obj(source).attached_to == Some(affected)
}

/// "You control enchanted creature."
static MIND: CardDef = CardDef {
    name: "Leave Test Mind",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(cantrip_mechanics::targets::CREATURE),
    statics: &[control_changing(enchanted, "")],
    ..CardDef::BLANK
};

const P1: PlayerId = PlayerId(0);
const P2: PlayerId = PlayerId(1);
const P3: PlayerId = PlayerId(2);

fn loss_with_a_stolen_creature(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    let Event::GameLost { player, .. } = *event else { return None };
    q.state().battlefield().iter().any(|&id| q.chars(id).name == BEAR.name && q.controller(id) == player)
        .then(|| Fired::about(Target::Player(player))).flatten()
}

fn no_effect(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }

static LOSS_WITNESS: CardDef = CardDef {
    name: "Pre-loss control witness", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(when(EventMask::GAME_OVER, loss_with_a_stolen_creature), no_effect,
        "Whenever a player loses the game, if they controlled a Leave Test Bear just before losing, observe that loss.")],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId, controller: PlayerId) -> ObjectId {
    let card = game.cards.id_of(def);
    let id = game.state.push_obj(Obj::new(card, owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| {
        o.controller = controller;
        o.timestamp = timestamp;
    });
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

/// A remaining player's Aura controlled by the leaver exposes both halves of CR
/// 800.4a: the Bear returns before any post-loss query, and the old controller is
/// still visible to a loss ability's look-back (CR 603.10f).
fn loss_settles_control_and_keeps_the_pre_loss_board(reason: LossReason) {
    let table = Box::leak(Box::new(CardTable::new([&MIND, &BEAR, &LOSS_WITNESS])));
    let deck = vec![table.id_of(&BEAR); 10];
    let mut game = Game::new(table, &Setup { decks: vec![deck; 3], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mind = put(&mut game, &MIND, P1, P3);
    let witness = put(&mut game, &LOSS_WITNESS, P1, P1);
    put(&mut game, &LOSS_WITNESS, P3, P3);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, P3);
    match reason {
        LossReason::Conceded => crate::ops::concede(&mut cx, P3),
        LossReason::Effect => crate::ops::lose_game(&mut cx, P3),
        LossReason::Life => {
            cx.st().update_player(P3, |p| p.life = 0);
            assert!(crate::settle::state_based_actions(&mut cx).unwrap());
        }
        _ => unreachable!(),
    }
    assert!(cx.s().player(P3).lost);
    assert!(!cx.s().player(P1).lost && !cx.s().player(P2).lost);
    assert_eq!(cx.s().obj(bear).controller, P2);
    assert!(cx.s().battlefield().contains(&bear));
    assert!(!cx.s().battlefield().contains(&mind));
    assert!(cx.s().triggers.iter().any(|t| t.source == witness && t.controller == P1 && t.subject == Target::Player(P3)),
        "loss trigger must see the Bear's pre-loss controller for {reason:?}");
    assert!(cx.g.log.iter().any(|r| r.event == Event::GameLost { player: P3, reason }));
    assert!(!cx.s().triggers.iter().any(|t| t.controller == P3), "the leaver's own loss triggers must be removed");
}

#[test]
fn concession_settles_control_and_keeps_pre_loss_trigger_state() {
    loss_settles_control_and_keeps_the_pre_loss_board(LossReason::Conceded);
}

#[test]
fn effect_loss_settles_control_and_keeps_pre_loss_trigger_state() {
    loss_settles_control_and_keeps_the_pre_loss_board(LossReason::Effect);
}

#[test]
fn sba_loss_settles_control_and_keeps_pre_loss_trigger_state() {
    loss_settles_control_and_keeps_the_pre_loss_board(LossReason::Life);
}

#[test]
fn simultaneous_losses_are_recorded_before_control_and_designation_changes() {
    let p4 = PlayerId(3);
    let table = Box::leak(Box::new(CardTable::new([&MIND, &BEAR, &LOSS_WITNESS])));
    let deck = vec![table.id_of(&BEAR); 10];
    let mut game = Game::new(table, &Setup { decks: vec![deck; 4], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mind = put(&mut game, &MIND, P1, P3);
    let witness = put(&mut game, &LOSS_WITNESS, P1, P1);
    put(&mut game, &LOSS_WITNESS, P3, P3);
    put(&mut game, &LOSS_WITNESS, p4, p4);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    cx.st().update_player(P3, |p| { p.life = 0; p.monarch = true; });
    cx.st().update_player(p4, |p| { p.life = 0; p.initiative = true; });
    let start = cx.g.log.len();
    assert!(crate::settle::state_based_actions(&mut cx).unwrap());
    assert!(cx.s().player(P3).lost && cx.s().player(p4).lost);
    assert!(!cx.s().player(P1).lost && !cx.s().player(P2).lost);
    assert_eq!(cx.s().obj(bear).controller, P2);
    assert!(cx.s().player(P1).monarch && cx.s().player(P1).initiative);
    assert!(cx.s().triggers.iter().any(|t| t.source == witness && t.subject == Target::Player(P3)));
    assert!(!cx.s().triggers.iter().any(|t| t.controller == P3 || t.controller == p4));
    let events = &cx.g.log[start..];
    let losses: Vec<_> = events.iter().enumerate().filter_map(|(i, r)| matches!(r.event, Event::GameLost { .. }).then_some(i)).collect();
    assert_eq!(losses.len(), 2);
    for (i, record) in events.iter().enumerate() {
        if matches!(record.event, Event::ControlChanged { permanent, .. } if permanent == bear)
            || matches!(record.event, Event::BecameMonarch { .. } | Event::TookInitiative { .. }) {
            assert!(i > losses[1], "all losses precede their consequences");
        }
    }
}

#[test]
fn the_last_opponent_leaving_settles_control_and_ends_the_game() {
    let table = Box::leak(Box::new(CardTable::new([&MIND, &BEAR, &LOSS_WITNESS])));
    let deck = vec![table.id_of(&BEAR); 10];
    let mut game = Game::new(table, &Setup { decks: vec![deck; 2], ..Default::default() });
    let bear = put(&mut game, &BEAR, P1, P1);
    let mind = put(&mut game, &MIND, P1, P2);
    put(&mut game, &LOSS_WITNESS, P1, P1);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, P2);
    crate::ops::concede(&mut cx, P2);
    assert!(cx.s().is_over());
    assert_eq!(cx.s().obj(bear).controller, P1);
}

/// P3's Anthem is before P3's Bear in the battlefield's list, so leaving one at a time would take the Bear's row with the
/// Anthem gone; together, the Bear's row is the 3/3 it last was. P1's Bear that P3 controls by an effect goes back to P1
/// and stays; P1's other Bear, which P3 controls with no effect to end, is exiled after P3's objects have left, a 2/2 then.
#[test]
fn a_leaving_players_objects_leave_together_and_what_they_still_control_is_exiled_after() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&ANTHEM, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let anthem = put(&mut game, &ANTHEM, P3, P3);
    let own_bear = put(&mut game, &BEAR, P3, P3);
    let stolen = put(&mut game, &BEAR, P1, P1);
    let kept = put(&mut game, &BEAR, P1, P3);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, anthem, stolen, EffectKind::Control(P3), Duration::Indefinite);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(stolen).controller, P3);
    assert_eq!(cx.chars(own_bear).power, Some(3));
    crate::ops::leave_game(&mut cx, P3);
    let s = cx.s();
    assert!(s.obj(anthem).zone.is(ZoneKind::Gone) && s.obj(own_bear).zone.is(ZoneKind::Gone));
    assert_eq!(s.last_known(own_bear).map(|k| (k.power, k.toughness)), Some((Some(3), Some(3))));
    // The effect that gave P3 control ended: P1's Bear is P1's again, still on the battlefield.
    assert!(s.battlefield().contains(&stolen));
    assert_eq!(s.obj(stolen).controller, P1);
    // What P3 still controlled is exiled, as it was once P3's objects had left.
    assert!(!s.battlefield().contains(&kept));
    assert_eq!(s.last_known(kept).map(|k| k.power), Some(Some(2)));
    assert_eq!(s.zone(Zone::exile(P1)).len(), 1);
}

/// CR 800.4a ends every effect that gives the leaving player control, a static ability's too: P3 controls P1's Mind
/// Control-style Aura on P2's Bear, so controls the Bear. As P3 leaves, the Aura no longer gives them the Bear, which goes
/// back to P2 and stays on the battlefield; the Aura, still P3's, is exiled to its owner's exile.
#[test]
fn a_static_control_ability_stops_giving_the_leaving_player_control() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&MIND, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mind = put(&mut game, &MIND, P1, P3);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, P3);
    cx.st().update_player(P3, |p| p.lost = true);
    crate::ops::leave_game(&mut cx, P3);
    let s = cx.s();
    assert!(s.battlefield().contains(&bear));
    assert_eq!(s.obj(bear).controller, P2);
    assert!(!s.battlefield().contains(&mind));
    assert_eq!(s.zone(Zone::exile(P1)).len(), 1);
}

/// CR 800.4b: an object doesn't change to the control of a player who has left the game, so an effect that would give them
/// control after they left never begins ("that player gains control of Kain" after a three-player game's Kain dealt them
/// lethal damage). The Bear stays P2's, and `gain_control` says the player did not gain it ("if they do", CR 608.2c).
#[test]
fn a_player_who_has_left_gains_no_control() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mut cx = cx(&mut game);
    cx.st().update_player(P3, |p| p.lost = true);
    crate::ops::leave_game(&mut cx, P3);
    assert!(!cantrip_api::Game::gain_control(&mut cx, bear, bear, P3, Duration::Indefinite));
    let s = cx.s();
    assert!(s.battlefield().contains(&bear));
    assert_eq!(s.obj(bear).controller, P2);
    assert!(!s.effects.iter().any(|e| e.kind == EffectKind::Control(P3)));
}

/// The other side of it: a player still in the game gains control, and `gain_control` says so.
#[test]
fn a_player_still_in_the_game_gains_control() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mut cx = cx(&mut game);
    assert!(cantrip_api::Game::gain_control(&mut cx, bear, bear, P3, Duration::Indefinite));
    assert_eq!(cx.s().obj(bear).controller, P3);
}

static LEGEND: CardDef = CardDef {
    name: "Leave Test Legend",
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    ..BEAR
};

/// CR 704.3, 800.4a: a player losing in a state-based check is not asked the legend rule in that same check. The two
/// same-named legends they control (others own them: two Mind Controls) are exiled as they leave, neither put into a
/// graveyard by the legend rule.
#[test]
fn a_player_losing_in_a_check_is_not_asked_the_legend_rule() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LEGEND])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![table.id_of(&LEGEND); 10]; 3], ..Default::default() });
    let first = put(&mut game, &LEGEND, P1, P3);
    let second = put(&mut game, &LEGEND, P2, P3);
    let mut cx = cx(&mut game);
    cx.st().update_player(P3, |p| p.life = 0);
    assert!(crate::settle::state_based_actions(&mut cx).unwrap(), "no legend-rule question is asked of the leaver");
    assert!(cx.s().player(P3).lost && !cx.s().player(P1).lost && !cx.s().player(P2).lost);
    assert_eq!((cx.s().zone(Zone::exile(P1)).len(), cx.s().zone(Zone::exile(P2)).len()), (1, 1), "both are exiled as still the leaver's");
    assert!(!cx.s().battlefield().contains(&first) && !cx.s().battlefield().contains(&second));
    assert!(!cx.g.log.iter().any(|r| matches!(r.event, Event::ZoneChange { to, .. } if to.is(ZoneKind::Graveyard))),
        "neither legend is put into a graveyard by a legend rule asked of a player leaving in the same check");
}


/// CR 903.9a, 800.4a: a commander's owner who loses in a state-based check is not asked whether it returns to the
/// command zone in that same check; it leaves the game with them.
#[test]
fn a_player_losing_in_a_check_is_not_asked_to_return_their_commander() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![table.id_of(&BEAR); 10]; 3], commander: true, ..Default::default() });
    let leader = game.state.push_obj(Obj::new(table.id_of(&BEAR), P3, Zone::graveyard(P3)));
    game.state.zone_push(Zone::graveyard(P3), leader);
    game.state.update_obj(leader, |o| { o.commander = true; o.commander_return_pending = true; });
    let mut cx = cx(&mut game);
    cx.st().update_player(P3, |p| p.life = 0);
    assert!(crate::settle::state_based_actions(&mut cx).unwrap(), "no command-zone question is asked of the leaver");
    assert!(cx.s().player(P3).lost && !cx.s().player(P1).lost && !cx.s().player(P2).lost);
    assert!(cx.s().zone(Zone::COMMAND).is_empty());
    assert!(cx.s().obj(leader).zone.is(ZoneKind::Gone));
}

static SIEGE: CardDef = CardDef {
    name: "Leave Test Siege",
    types: TypeSet::of(&[CardType::Battle]),
    subtypes: SubtypeSet::of(&[Subtype::Siege]),
    defense: Some(5),
    ..CardDef::BLANK
};

/// CR 704.5x, 800.4a: a Siege whose protector can't protect it (here its own controller, after a control change) has a new
/// protector chosen, and a player losing in that same check is not among the choices: with the leaver left out there is
/// one candidate, so its controller is asked nothing.
#[test]
fn a_player_losing_in_a_check_cannot_be_chosen_to_protect_a_battle() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&SIEGE])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![table.id_of(&SIEGE); 10]; 3], ..Default::default() });
    let siege = put(&mut game, &SIEGE, P1, P1);
    game.state.update_obj(siege, |o| o.marks.protector = Some(P1));
    let mut cx = cx(&mut game);
    crate::counters::put(&mut cx, siege, CounterKind::Defense, 5, P1, crate::counters::How::Effect);
    cx.st().update_player(P3, |p| p.life = 0);
    assert!(crate::settle::state_based_actions(&mut cx).unwrap(), "no protector question is asked between P2 and the leaver");
    assert!(cx.s().player(P3).lost && !cx.s().player(P2).lost);
    assert_eq!(cx.s().obj(siege).marks.protector, Some(P2));
}

static ASKED: Question = Question::new("Pay", "pay {cost} for {subject}?", "", "Pay {cost}", "Decline");

/// CR 800.4a, 400.7: a question asked after a player conceded in the same run, listed before they left, offers nothing that
/// left with them and no player who left (`cantrip_state::without_departed`).
#[test]
fn a_question_listed_before_a_concession_offers_nothing_that_left() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![table.id_of(&BEAR); 10]; 3], ..Default::default() });
    let mine = put(&mut game, &BEAR, P1, P1);
    let theirs = put(&mut game, &BEAR, P3, P3);
    let source = put(&mut game, &BEAR, P1, P1);
    let stolen = put(&mut game, &BEAR, P2, P3);
    crate::ops::concede(&mut cx(&mut game), P3);
    let s = &game.state;
    assert!(cantrip_state::departed(s, theirs) && !cantrip_state::departed(s, mine));
    assert!(!s.battlefield().contains(&stolen) && cantrip_state::departed(s, stolen), "what the leaver controlled is exiled as they leave");
    let cards = |cards: Vec<ObjectId>, rule: Selection| RequestKind::ChooseCards { source, purpose: Purpose::Choose, cards, rule: Box::new(rule) };
    // Exactly two of a list one of which left: as many as can be, from what is left, the rule's own list too.
    let rule = Selection { among: Some(vec![mine, theirs]), ..Selection::new(Count::Exactly(2)) };
    let Ok(RequestKind::ChooseCards { cards: left, rule, .. }) = cantrip_state::without_departed(s, cards(vec![mine, theirs], rule), false) else { panic!() };
    assert_eq!((left, rule.among, rule.count), (vec![mine], Some(vec![mine]), Count::AsManyAsCan(2)));
    // Exactly one with one left is still exactly one.
    let Ok(RequestKind::ChooseCards { rule, .. }) = cantrip_state::without_departed(s, cards(vec![mine, theirs], Selection::new(Count::Exactly(1))), false) else { panic!() };
    assert_eq!(rule.count, Count::Exactly(1));
    // Nothing left: answered with nothing, unasked.
    assert_eq!(cantrip_state::without_departed(s, cards(vec![theirs], Selection::new(Count::Exactly(1))), false).err(), Some(Answer::Cards(Vec::new())));
    // A target slot that needs more than is left is given up where it may be (CR 601.2c), and takes what there is where not.
    let target = |least| RequestKind::ChooseTarget { source, purpose: Purpose::Targets, candidates: vec![Target::Object(mine), Target::Object(theirs), Target::Player(P3)],
        optional: false, least, count: 3, any_number: false, label: "", slot_player: None, together: None, replacing: None };
    assert_eq!(cantrip_state::without_departed(s, target(2), true).err(), Some(Answer::Cancel));
    let Ok(RequestKind::ChooseTarget { candidates, least, count, .. }) = cantrip_state::without_departed(s, target(2), false) else { panic!() };
    assert_eq!((candidates, least, count), (vec![Target::Object(mine)], 1, 1));
    let none = RequestKind::ChooseTarget { source, purpose: Purpose::Choose, candidates: vec![Target::Object(theirs), Target::Player(P3)],
        optional: false, least: 1, count: 1, any_number: false, label: "", slot_player: None, together: None, replacing: None };
    assert_eq!(cantrip_state::without_departed(s, none, false).err(), Some(Answer::Targets(Vec::new())));
    // A player who left is no player to choose.
    let player = RequestKind::ChoosePlayer { source, candidates: vec![P2, P3], verb: "", amount: 0, subject: None };
    let Ok(RequestKind::ChoosePlayer { candidates, .. }) = cantrip_state::without_departed(s, player, false) else { panic!() };
    assert_eq!(candidates, vec![P2]);
    // A payment for an object that left is not made; one for an object still here is asked.
    let pay = |subject| RequestKind::UnlessPays { source, subject, total: ManaCost::ZERO, remaining: ManaCost::ZERO, life: 0, life_payable: false,
        question: &ASKED, options: Vec::new(), x: None, times: None, removes: None };
    assert_eq!(cantrip_state::without_departed(s, pay(theirs), false).err(), Some(Answer::Pass));
    assert!(cantrip_state::without_departed(s, pay(mine), false).is_ok());
}

/// CR 104.3a, 800.4a: a concession comes at the question it was made at, not earlier. A question to a player who had
/// already left, answered for them unasked, is no question of the run's, so P2's concession at this point waits for the
/// question put to a player still in the game.
#[test]
fn a_concession_waits_for_a_question_put_to_a_player_still_in_the_game() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![table.id_of(&BEAR); 10]; 4], ..Default::default() });
    let source = put(&mut game, &BEAR, P1, P1);
    let p4 = PlayerId(3);
    crate::ops::concede(&mut cx(&mut game), p4);
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Settle));
    let frame = game.state.agenda.len() - 1;
    game.state.update_frame(frame, |f| f.conceded.push((0, P2)));
    let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    let pay = RequestKind::UnlessPays { source, subject: source, total: ManaCost::ZERO, remaining: ManaCost::ZERO, life: 0, life_payable: false,
        question: &ASKED, options: Vec::new(), x: None, times: None, removes: None };
    assert_eq!(cx.ask_core(p4, pay, false).ok(), Some(Answer::Pass), "a cost of a player who left is not paid, unasked");
    assert!(!cx.s().player(P2).lost, "the concession is not made at a question no one is asked");
    let ask = RequestKind::ChoosePlayer { source, candidates: vec![P2, P3], verb: "", amount: 0, subject: None };
    assert!(cx.ask_core(P1, ask, false).is_err(), "P1 is asked");
    assert!(cx.s().player(P2).lost);
    let Some(Request { kind: RequestKind::ChoosePlayer { candidates, .. }, .. }) = &cx.s().agenda[frame].pending else { panic!() };
    assert_eq!(candidates, &vec![P3], "the player who conceded is no longer a candidate");
}

/// The same for every other kind of question that lists objects or players: each loses what left (CR 800.4a, 400.7), and
/// the lists kept in step with them lose the same entries.
#[test]
fn every_kind_of_question_listed_before_a_concession_loses_what_left() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![table.id_of(&BEAR); 10]; 3], ..Default::default() });
    let mine = put(&mut game, &BEAR, P1, P1);
    let theirs = put(&mut game, &BEAR, P3, P3);
    let source = put(&mut game, &BEAR, P1, P1);
    crate::ops::concede(&mut cx(&mut game), P3);
    let s = &game.state;
    let filter = |kind: RequestKind| cantrip_state::without_departed(s, kind, false);
    let both = vec![mine, theirs];
    let choice = |objects: Vec<ObjectId>| CostPaymentChoice { label: String::new(), objects, kind: CostPaymentKind::Sacrifice };
    let Ok(RequestKind::ChooseCostPayment { choices, .. }) = filter(RequestKind::ChooseCostPayment { source, choices: vec![choice(vec![mine]), choice(vec![theirs])] }) else { panic!() };
    assert_eq!(choices, vec![choice(vec![mine])]);
    assert_eq!(cantrip_state::without_departed(s, RequestKind::ChooseCostPayment { source, choices: vec![choice(vec![theirs])] }, true).err(), Some(Answer::Cancel));
    assert_eq!(filter(RequestKind::ChooseCostPayment { source, choices: vec![choice(vec![theirs])] }).err(), Some(Answer::Pass));
    let options = vec![ActionOption::on(ActionKind::ActivateAbility, mine, 0), ActionOption::on(ActionKind::ActivateAbility, theirs, 0)];
    let Ok(RequestKind::Priority { options: left }) = filter(RequestKind::Priority { options }) else { panic!() };
    assert_eq!(left.iter().map(|o| o.source).collect::<Vec<_>>(), vec![Some(mine)]);
    let search = RequestKind::SearchLibrary { source, library: both.clone(), rule: Box::new(Selection::new(Count::Exactly(2))), destination: SearchDestination::Hand };
    let Ok(RequestKind::SearchLibrary { library, rule, .. }) = filter(search) else { panic!() };
    assert_eq!((library, rule.count), (vec![mine], Count::AsManyAsCan(2)));
    let divided = |optional| RequestKind::ChooseDividedTargets { source, candidates: vec![Target::Object(theirs), Target::Player(P3)], total: 2, max_targets: 2, optional, trigger: None };
    assert_eq!(cantrip_state::without_departed(s, divided(false), true).err(), Some(Answer::Cancel));
    assert_eq!(cantrip_state::without_departed(s, divided(true), true).err(), Some(Answer::Divided(Vec::new())));
    let divided = RequestKind::ChooseDividedTargets { source, candidates: vec![Target::Object(mine), Target::Player(P3)], total: 2, max_targets: 2, optional: false, trigger: None };
    let Ok(RequestKind::ChooseDividedTargets { candidates, max_targets, .. }) = filter(divided) else { panic!() };
    assert_eq!((candidates, max_targets), (vec![Target::Object(mine)], 1));
    let Ok(RequestKind::ChooseStartingPlayer { candidates }) = filter(RequestKind::ChooseStartingPlayer { candidates: vec![P1, P3] }) else { panic!() };
    assert_eq!(candidates, vec![P1]);
    assert_eq!(filter(RequestKind::ChoosePlayer { source, candidates: vec![P3], verb: "", amount: 0, subject: None }).err(), Some(Answer::Pass));
    let Ok(RequestKind::TapForPower { cards, powers, .. }) = filter(RequestKind::TapForPower { source, purpose: Purpose::Choose, cards: vec![theirs, mine], powers: vec![5, 2], minimum: 2, up_to: false }) else { panic!() };
    assert_eq!((cards, powers), (vec![mine], vec![2]));
    let Ok(RequestKind::CollectEvidence { cards, values, .. }) = filter(RequestKind::CollectEvidence { source, cards: vec![mine, theirs], values: vec![1, 3], minimum: 1, symbols: None }) else { panic!() };
    assert_eq!((cards, values), (vec![mine], vec![1]));
    let Ok(RequestKind::Craft { cards, .. }) = filter(RequestKind::Craft { source, cards: both.clone(), minimum: 1, maximum: 1, quality: "" }) else { panic!() };
    assert_eq!(cards, vec![mine]);
    let Ok(RequestKind::DistributeCards { cards, .. }) = filter(RequestKind::DistributeCards { source, cards: both.clone(), destinations: Vec::new(), may_shuffle: false }) else { panic!() };
    assert_eq!(cards, vec![mine]);
    let pool = |object| CounterPool { object, kind: CounterKind::PlusOne, count: 1 };
    let Ok(RequestKind::RemoveCounters { pools, .. }) = filter(RequestKind::RemoveCounters { source, pools: vec![pool(mine), pool(theirs)], total: 1, up_to: false }) else { panic!() };
    assert_eq!(pools, vec![pool(mine)]);
    let cost = |attacker, target| AttackCost { attacker, target, cost: ManaCost::ZERO, source };
    let attack = RequestKind::DeclareAttackers { eligible: both.clone(), players: vec![P2, P3], planeswalkers: both.clone(), battles: both.clone(),
        requirements: Default::default(), costs: vec![cost(mine, Target::Player(P2)), cost(theirs, Target::Player(P2)), cost(mine, Target::Player(P3))], can_pay: 0 };
    let Ok(RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, costs, .. }) = filter(attack) else { panic!() };
    assert_eq!((eligible, players, planeswalkers, battles, costs), (vec![mine], vec![P2], vec![mine], vec![mine], vec![cost(mine, Target::Player(P2))]));
    let block = RequestKind::DeclareBlockers { eligible: both.clone(), attackers: both.clone(), allowed: vec![(mine, mine), (mine, theirs), (theirs, mine)], requirements: Default::default() };
    let Ok(RequestKind::DeclareBlockers { eligible, attackers, allowed, .. }) = filter(block) else { panic!() };
    assert_eq!((eligible, attackers, allowed), (vec![mine], vec![mine], vec![(mine, mine)]));
    let division = |source, creatures: Vec<ObjectId>, defender| DamageDivision { source, banding: false, damage: 2, lethal: creatures.iter().map(|_| 2).collect(), creatures,
        may_assign_to_defender: true, defender, suggested: Vec::new(), suggested_to_defender: 0 };
    let damage = RequestKind::AssignCombatDamage { divisions: vec![division(mine, vec![theirs, mine], Target::Player(P3)), division(mine, vec![theirs], Target::Player(P3)),
        division(mine, vec![theirs], Target::Player(P2)), division(theirs, vec![mine], Target::Player(P2))] };
    let Ok(RequestKind::AssignCombatDamage { divisions }) = filter(damage) else { panic!() };
    let left: Vec<_> = divisions.iter().map(|d| (d.creatures.clone(), d.lethal.clone(), d.may_assign_to_defender)).collect();
    assert_eq!(left, vec![(vec![mine], vec![2], false), (Vec::new(), Vec::new(), true)]);
    let Ok(RequestKind::DiscardToHandSize { count, hand }) = filter(RequestKind::DiscardToHandSize { count: 2, hand: both.clone() }) else { panic!() };
    assert_eq!((count, hand), (1, vec![mine]));
}
