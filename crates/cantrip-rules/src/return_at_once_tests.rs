//! What was exiled "until [a permanent] leaves the battlefield" returns immediately after it leaves (CR 610.3), not once the
//! state-based actions have stopped doing anything: before the next check, which sees it back, over test cards.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{ExileLink, Frame, FrameKind};

static WARDEN: CardDef = CardDef {
    name: "Return Test Warden",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

static SHELL: CardDef = CardDef {
    name: "Return Test Shell",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn enchant_creature(q: &dyn Query, _: &Asker, target: Target) -> bool {
    matches!(target, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

static CHARM: CardDef = CardDef {
    name: "Return Test Charm",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(enchant_creature)),
    ..CardDef::BLANK
};

static CAPTIVE: CardDef = CardDef {
    name: "Return Test Captive",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn game(seats: usize) -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&WARDEN, &SHELL, &CHARM, &CAPTIVE])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(); seats], ..Default::default() });
    // Nobody has drawn from an empty library yet: only what a test does is checked.
    for p in 0..seats {
        game.state.update_player(PlayerId(p as u8), |pl| pl.drew_from_empty = false);
    }
    (game, table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

/// A Captive of `owner`'s exiled from the battlefield until `by` leaves it.
fn exile_until(game: &mut Game, table: &CardTable, owner: PlayerId, by: ObjectId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(&CAPTIVE), owner, Zone::exile(owner)));
    game.state.zone_push(Zone::exile(owner), id);
    let text = game.state.obj(by).card;
    game.state.update_obj(id, |o| o.exiled = Some(ExileLink { by, text, acquired: Acquired::OWN, from: Zone::BATTLEFIELD, until_leaves: true, return_by: None, encoded: false }));
    id
}

/// The state-based actions performed in a settling frame of the game's own, as the rules perform them.
fn state_based_actions(game: &mut Game) {
    game.state.frame_push(Frame::new(FrameKind::Settle));
    let frame = game.state.agenda.len() - 1;
    let mut cx = Cx { g: game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    assert!(crate::settle::state_based_actions_alone(&mut cx).expect("nothing is asked"));
}

/// Where in the history the object `id` moved from `from` to `to`.
fn moved(game: &Game, id: ObjectId, from: ZoneKind, to: ZoneKind) -> usize {
    game.log
        .iter()
        .position(|r| matches!(r.event, Event::ZoneChange { old, from: f, to: t, .. } if old == id && f.is(from) && t.is(to)))
        .unwrap_or_else(|| panic!("{id:?} never moved from {from:?} to {to:?}"))
}

/// The Warden, a 0/0, dies to the first check (CR 704.5f); the Charm enchanting it goes to the graveyard only at the second
/// (CR 704.5m), and the Captive it held is back on the battlefield between the two, under its owner's control (CR 610.3c).
#[test]
fn a_card_exiled_until_a_permanent_that_dies_to_a_state_based_action_returns_before_the_next_check() {
    let (mut game, table) = game(2);
    let warden = put(&mut game, table, &WARDEN, PlayerId(0));
    let shell = put(&mut game, table, &SHELL, PlayerId(0));
    let charm = put(&mut game, table, &CHARM, PlayerId(0));
    game.state.update_obj(charm, |o| o.attached_to = Some(warden));
    let captive = exile_until(&mut game, table, PlayerId(1), warden);
    state_based_actions(&mut game);
    let died = moved(&game, warden, ZoneKind::Battlefield, ZoneKind::Graveyard);
    let returned = moved(&game, captive, ZoneKind::Exile, ZoneKind::Battlefield);
    let fell_off = moved(&game, charm, ZoneKind::Battlefield, ZoneKind::Graveyard);
    assert!(died < returned && returned < fell_off, "died at {died}, returned at {returned}, the Charm fell off at {fell_off}");
    let back = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == table.id_of(&CAPTIVE)).expect("the Captive is back");
    assert_eq!(game.state.obj(back).controller, PlayerId(1));
    assert!(game.state.battlefield().contains(&shell));
}

/// Two players lose to the same check (CR 704.5a) and leave the game with everything they own (CR 800.4a): the first's
/// Warden held a Captive of the third player's and one of the second loser's, the second's Warden another of the third
/// player's. The check is one event (CR 704.3), so the returns come once it is all done, both losers gone, and together
/// (CR 610.3d): the third player's two Captives are back after both losses, and before the next check puts the fourth
/// player's Charm, that enchanted the first Warden, into its owner's graveyard; the second loser's Captive is not
/// returned between the two losses, under a player about to leave, but leaves the game with its owner.
#[test]
fn cards_exiled_until_permanents_whose_owners_lose_together_return_once_the_check_is_done() {
    let (mut game, table) = game(4);
    let first = put(&mut game, table, &SHELL, PlayerId(0));
    let second = put(&mut game, table, &SHELL, PlayerId(1));
    let charm = put(&mut game, table, &CHARM, PlayerId(3));
    game.state.update_obj(charm, |o| o.attached_to = Some(first));
    let theirs = [exile_until(&mut game, table, PlayerId(2), first), exile_until(&mut game, table, PlayerId(2), second)];
    let losers = exile_until(&mut game, table, PlayerId(1), first);
    game.state.update_player(PlayerId(0), |p| p.life = 0);
    game.state.update_player(PlayerId(1), |p| p.life = 0);
    state_based_actions(&mut game);
    let lost = |p: u8| game.log.iter().position(|r| matches!(r.event, Event::GameLost { player, .. } if player == PlayerId(p))).expect("lost");
    let both_lost = lost(0).max(lost(1));
    let returned = theirs.map(|c| moved(&game, c, ZoneKind::Exile, ZoneKind::Battlefield));
    let fell_off = moved(&game, charm, ZoneKind::Battlefield, ZoneKind::Graveyard);
    assert!(returned.iter().all(|&r| both_lost < r && r < fell_off), "both lost by {both_lost}, returned at {returned:?}, the Charm fell off at {fell_off}");
    let entered = game.log.iter().any(|r| matches!(r.event, Event::ZoneChange { old, to, .. } if old == losers && to.is(ZoneKind::Battlefield)));
    assert!(!entered, "the second loser's Captive never came back");
    assert!(game.state.obj(losers).zone.is(ZoneKind::Gone));
}
