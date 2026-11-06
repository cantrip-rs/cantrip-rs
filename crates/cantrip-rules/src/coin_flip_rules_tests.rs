//! Coin flips whose result an effect states (CR 705.3), over a test card with Edgar, King of Figaro's rule: the rule is
//! asked once of a set of coins flipped together, and a coin flip for heads it fixes is won as well.

use crate::{Cx, Frame, FrameKind, Game, Setup};
use cantrip_api::*;

/// "The first time you flip one or more coins each turn, those coins come up heads and you win those flips."
fn first_flips_each_turn(q: &dyn Query, me: &Asker, player: PlayerId) -> bool {
    player == me.controller
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::CoinFlipped { player: p, .. } if p == player))
}

static COIN: CardDef = CardDef { name: "Coin Test Relic", types: TypeSet::of(&[CardType::Artifact]), coin_flip_rules: &[first_flips_each_turn], ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// A game with the Relic on `YOU`'s battlefield, every die forced to show 2 (a lost flip, or tails), and a frame to settle in.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&COIN])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let id = game.state.push_obj(Obj::new(table.id_of(&COIN), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    game.state.force_rolls(&[2; 10]);
    game.state.frame_push(Frame::new(FrameKind::Settle));
    game
}

fn flips(cx: &Cx, player: PlayerId) -> Vec<CoinFlip> {
    cx.history()
        .this_turn()
        .iter()
        .filter_map(|r| match r.event {
            Event::CoinFlipped { player: p, result } if p == player => Some(result),
            _ => None,
        })
        .collect()
}

/// Three coins flipped together are one set: the rule, asked once before any is flipped, makes every one heads and won.
/// The next flip that turn is the second time, and comes up as the die says.
#[test]
fn coins_flipped_together_are_all_fixed_and_a_later_flip_is_not() {
    let mut game = game();
    let frame = game.state.agenda.len() - 1;
    let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, damage: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    assert_eq!(cantrip_api::Game::flip_coins_for_heads(&mut cx, YOU, 3), vec![true, true, true]);
    assert_eq!(flips(&cx, YOU), vec![CoinFlip::WonHeads; 3]);
    assert!(!cantrip_api::Game::flip_coin(&mut cx, YOU));
    assert_eq!(flips(&cx, YOU).last(), Some(&CoinFlip::Lost));
}

/// CR 705.3: a flip for heads the rule fixes is won as well as heads - `won()` is what "whenever you win a coin flip" reads.
/// The rule's controller's opponent flips as the die says.
#[test]
fn a_fixed_flip_for_heads_is_won_and_an_opponents_flip_is_not_fixed() {
    let mut game = game();
    let frame = game.state.agenda.len() - 1;
    let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, damage: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    assert!(cantrip_api::Game::flip_coin_for_heads(&mut cx, YOU));
    let mine = flips(&cx, YOU);
    assert_eq!(mine, vec![CoinFlip::WonHeads]);
    assert!(mine[0].won() && mine[0].heads());
    assert!(!cantrip_api::Game::flip_coin_for_heads(&mut cx, THEM));
    assert_eq!(flips(&cx, THEM), vec![CoinFlip::Tails]);
}
