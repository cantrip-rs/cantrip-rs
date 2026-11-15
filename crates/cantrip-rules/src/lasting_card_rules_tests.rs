//! Rules a resolving spell or ability gives an open set of cards rather than permanents (CR 611.2c, `EffectKind::LastingRule`):
//! "[that player] can't play [that card]" (CR 101.2, 601.3, 305.1, 701.18b), which forbids the player its words named to cast
//! the card or play it as a land, with priority and as an effect resolves, whatever allows it - and no other player; and
//! "plays with [that card] revealed in their hand" (CR 701.20a), which makes the card public while it is in a hand, as a hand
//! a static ability reveals is (`hand_visibility`). Firestorm Phoenix's, over test cards.

use crate::actions::{cast_ways_during_resolution, could_play_land_during_resolution, priority_options, stack_item};
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "That card": the effect's source, as Firestorm Phoenix's set is the one card its source became.
fn that_card(_: &dyn Query, asker: &Asker, _: Target, id: ObjectId) -> bool {
    id == asker.source
}

const CANT_BE_PLAYED: u8 = 0;
const REVEALED: u8 = 1;

static RULES: CardDef = CardDef {
    name: "Lasting Card Rule Test Rules",
    types: TypeSet::of(&[CardType::Sorcery]),
    lasting_rules: &[
        LastingRule { text: "That player can't play that card.", rule: LastingRuleKind::CantBePlayed, applies: that_card },
        LastingRule { text: "That card is revealed in its owner's hand.", rule: LastingRuleKind::RevealedInHand, applies: that_card },
    ],
    ..CardDef::BLANK
};

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static BEAR: CardDef = CardDef {
    name: "Lasting Card Rule Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef { name: "Lasting Card Rule Test Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
/// "You may play a land from your graveyard", a way a permanent gives.
static PARAGON: CardDef = CardDef {
    name: "Lasting Card Rule Test Paragon",
    types: TypeSet::of(&[CardType::Enchantment]),
    granted_ways: &[GrantedWay { text: "Play a land from your graveyard.", cost: |_, _, _, _| Some(Cost::NONE), lands: true, permission: true, ..GrantedWay::CAST }],
    ..CardDef::BLANK
};

/// Your main phase, you holding priority with two bears and two lands in your hand and nothing else anywhere.
fn game() -> (Game, [ObjectId; 4]) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&RULES, &BEAR, &LAND, &PARAGON])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let cards = [put(&mut game, &BEAR, Zone::hand(YOU)), put(&mut game, &BEAR, Zone::hand(YOU)), put(&mut game, &LAND, Zone::hand(YOU)), put(&mut game, &LAND, Zone::hand(YOU))];
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, cards)
}

/// A card of yours in `zone`.
fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    id
}

/// The `index`th rule of `RULES` about `card`, its words naming `about` ("that player", or no one).
fn rule(game: &mut Game, index: u8, card: ObjectId, about: Target) {
    let timestamp = game.state.timestamp();
    let kind = EffectKind::LastingRule { card: game.cards.id_of(&RULES), index, about };
    game.state.effect_push(ContinuousEffect { source: card, controller: YOU, timestamp, affected: card, kind, duration: Duration::UntilNextTurnOf(YOU), words: Words::NONE });
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// What you are offered to do with each card with priority.
fn offered(game: &mut Game, kind: ActionKind, card: ObjectId) -> bool {
    priority_options(&mut cx(game), YOU).iter().any(|o| o.kind == kind && o.source == Some(card))
}

/// A resolving spell of yours, in whose resolution a card may be cast or played (CR 608.2g).
fn resolving(game: &mut Game) -> Cx<'_> {
    let rules = game.cards.id_of(&RULES);
    let id = game.state.push_obj(Obj::new(rules, YOU, Zone::STACK));
    game.state.zone_push(Zone::STACK, id);
    game.state.stack_push(stack_item(StackKind::Spell, id, AbilityRef { card: rules, index: 0 }, YOU, &[]));
    let serial = game.state.stack.last().expect("the spell").serial;
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
    let frame = game.state.agenda.len() - 1;
    Cx { g: game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// CR 101.2, 601.3, 305.1, 701.18b: the cards a rule binding you takes in can't be cast or played as a land by you, with
/// priority or as an effect resolves; their twins beside them in the hand still can.
#[test]
fn a_card_that_cant_be_played_is_neither_cast_nor_played_as_a_land() {
    let (mut game, [bear, other_bear, land, other_land]) = game();
    assert!(offered(&mut game, ActionKind::CastSpell, bear) && offered(&mut game, ActionKind::PlayLand, land), "nothing forbids them yet");
    rule(&mut game, CANT_BE_PLAYED, bear, Target::Player(YOU));
    rule(&mut game, CANT_BE_PLAYED, land, Target::Player(YOU));
    assert!(!offered(&mut game, ActionKind::CastSpell, bear), "the bear can't be cast");
    assert!(!offered(&mut game, ActionKind::PlayLand, land), "the land can't be played");
    assert!(offered(&mut game, ActionKind::CastSpell, other_bear), "another bear can");
    assert!(offered(&mut game, ActionKind::PlayLand, other_land), "another land can");
    let mut cx = resolving(&mut game);
    assert!(cast_ways_during_resolution(&mut cx, bear, YOU).is_empty(), "nor cast by an effect");
    assert!(!cast_ways_during_resolution(&mut cx, other_bear, YOU).is_empty());
    assert!(!could_play_land_during_resolution(&cx, land), "nor played by an effect");
    assert!(could_play_land_during_resolution(&cx, other_land));
}

/// CR 101.2, 701.18b: the rule binds only the player its words named. A player it doesn't name - you, for a rule about your
/// opponent, as Sen Triplets lets a player cast a card from another's hand - still casts the card and plays the land, with
/// priority and as an effect resolves; a rule that named no player binds every player.
#[test]
fn a_card_that_cant_be_played_by_another_player_is_still_yours_to_play() {
    let (mut game, [bear, _, land, _]) = game();
    rule(&mut game, CANT_BE_PLAYED, bear, Target::Player(THEM));
    rule(&mut game, CANT_BE_PLAYED, land, Target::Player(THEM));
    assert!(offered(&mut game, ActionKind::CastSpell, bear), "the rule binds them, not you");
    assert!(offered(&mut game, ActionKind::PlayLand, land));
    {
        let mut cx = resolving(&mut game);
        assert!(!cast_ways_during_resolution(&mut cx, bear, YOU).is_empty());
        assert!(could_play_land_during_resolution(&cx, land));
    }
    rule(&mut game, CANT_BE_PLAYED, bear, Target::None);
    assert!(!offered(&mut game, ActionKind::CastSpell, bear), "a rule naming no one binds you too");
}

/// CR 101.2, 305.1: a way a permanent gives to play lands from the graveyard doesn't reach a land the rule takes in there.
#[test]
fn a_land_that_cant_be_played_is_not_played_by_a_way_a_permanent_gives() {
    let (mut game, _) = game();
    put(&mut game, &PARAGON, Zone::BATTLEFIELD);
    let land = put(&mut game, &LAND, Zone::graveyard(YOU));
    let other_land = put(&mut game, &LAND, Zone::graveyard(YOU));
    rule(&mut game, CANT_BE_PLAYED, land, Target::Player(YOU));
    assert!(!offered(&mut game, ActionKind::PlayLand, land));
    assert!(offered(&mut game, ActionKind::PlayLand, other_land));
}

/// CR 701.20a: a card a rule keeps revealed in a hand is public, as a hand a static ability reveals is - one card of it, not
/// its twin: it is among the hands' public cards, public for the look-back as it moves (CR 603.10a), and the other seat is
/// shown it once, as it becomes public (`Event::Shown`), not again. A rule that it can't be played reveals nothing.
#[test]
fn a_card_revealed_in_a_hand_is_public_alone() {
    let (mut game, [bear, other_bear, land, _]) = game();
    assert!(game.revealed_hand_cards().is_empty());
    rule(&mut game, CANT_BE_PLAYED, land, Target::Player(YOU));
    assert!(game.revealed_hand_cards().is_empty(), "a card that can't be played is not revealed by that");
    rule(&mut game, REVEALED, bear, Target::Player(YOU));
    assert_eq!(game.revealed_hand_cards(), [bear], "{other_bear:?} stays hidden");
    assert_eq!(game.revealed_hands(), 0, "the hand is not public as a whole");
    assert!(crate::ops::seen_by_all(&cx(&mut game), bear, false));
    assert!(!crate::ops::seen_by_all(&cx(&mut game), other_bear, false));
    let shown = |game: &Game| game.log.iter().filter(|r| matches!(r.event, Event::Shown { player, card } if player == THEM && card == bear)).count();
    crate::hand_visibility::sync(&mut cx(&mut game));
    assert_eq!(shown(&game), 1, "shown to the other seat as it becomes public");
    crate::hand_visibility::sync(&mut cx(&mut game));
    assert_eq!(shown(&game), 1, "and not again");
    assert!(!game.log.iter().any(|r| matches!(r.event, Event::Shown { player: YOU, .. })), "its owner is shown nothing");
    // Put back into the library together, the one leaves a public hand and the other a hidden one.
    crate::ops::move_together(
        &mut cx(&mut game),
        &[crate::ops::Moving::to(bear, Zone::library(YOU)), crate::ops::Moving::to(other_bear, Zone::library(YOU))],
        ZoneReason::Other,
    );
    let left_public = |game: &Game, card: ObjectId| {
        game.log.iter().find_map(|r| match r.event {
            Event::ZoneChange { old, from_revealed_hand, .. } if old == card => Some(from_revealed_hand),
            _ => None,
        })
    };
    assert_eq!(left_public(&game, bear), Some(true));
    assert_eq!(left_public(&game, other_bear), Some(false));
}
