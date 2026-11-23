//! What a variant sets of a seat's start (`Setup::variant`) as the real opening plays it (CR 103.5): a player draws their
//! starting hand size for each hand and may mulligan until their opening hand would be zero cards. A scenario cannot show
//! a question that is not asked, so these count the questions.

use cantrip_engine::*;

/// A real deal of `seats` seats, Bot1 with a starting hand of `hand`.
fn deal(seats: usize, hand: u8) -> Game {
    let forest = card("Forest").expect("Forest");
    let setup = Setup {
        decks: vec![vec![forest; 40]; seats],
        seed: 3,
        shuffle: true,
        variant: vec![SeatVariant { starting_hand_size: Some(hand), ..SeatVariant::default() }],
        ..Setup::default()
    };
    let mut game = Game::new(cards(), &setup);
    game.run();
    game
}

/// Plays the opening with Bot1 going first and taking every mulligan it is offered, the others keeping, and Bot1 putting
/// whatever it must on the bottom: how many times Bot1 was asked, and its opening hand.
fn mulligan_all(game: &mut Game) -> (u32, usize) {
    let bot1 = PlayerId(0);
    let mut asked = 0;
    while let Some(request) = game.pending().cloned() {
        let answer = match &request.kind {
            RequestKind::ChooseStartingPlayer { .. } => Answer::Player(bot1),
            RequestKind::Mulligan { .. } if request.player == bot1 => {
                asked += 1;
                Answer::Yes
            }
            RequestKind::Mulligan { .. } => Answer::No,
            _ if game.state.turn.number == 0 && request.player == bot1 => {
                // The cards a keep puts on the bottom: all of Bot1's hand here.
                Answer::Cards(game.state.zone(Zone::hand(bot1)).to_vec())
            }
            _ => break,
        };
        game.answer(request.id, request.player, answer).expect("an answer the opening takes");
    }
    (asked, game.state.zone(Zone::hand(PlayerId(0))).len())
}

#[test]
fn a_duelist_mulligans_until_their_opening_hand_would_be_empty() {
    let mut game = deal(2, 2);
    let (asked, hand) = mulligan_all(&mut game);
    assert_eq!(asked, 2, "a starting hand of two allows two mulligans, and the player keeps after them");
    assert_eq!(hand, 0, "two mulligans put both cards of the third hand on the bottom");
    assert_eq!(game.state.player(PlayerId(1)).starting_hand_size, 7);
    assert_eq!(game.state.zone(Zone::hand(PlayerId(1))).len(), 7);
}

#[test]
fn a_multiplayer_first_mulligan_is_free_with_any_starting_hand() {
    // CR 103.5c: the first mulligan doesn't count, so a starting hand of two allows three.
    let mut game = deal(3, 2);
    let (asked, hand) = mulligan_all(&mut game);
    assert_eq!(asked, 3);
    assert_eq!(hand, 0);
}

#[test]
fn a_variant_starting_hand_is_drawn_and_its_maximum_kept() {
    let forest = card("Forest").expect("Forest");
    let setup = Setup {
        decks: vec![vec![forest; 40]; 2],
        variant: vec![
            SeatVariant { starting_life: Some(33), starting_hand_size: Some(9), maximum_hand_size: Some(11), emblems: Vec::new() },
            SeatVariant { starting_hand_size: Some(0), ..SeatVariant::default() },
        ],
        ..Setup::default()
    };
    let game = Game::new(cards(), &setup);
    let (bot1, bot2) = (game.state.player(PlayerId(0)), game.state.player(PlayerId(1)));
    assert_eq!((bot1.life, bot1.starting_life, bot1.starting_hand_size, bot1.maximum_hand_size), (33, 33, 9, 11));
    assert_eq!((bot2.life, bot2.starting_hand_size, bot2.maximum_hand_size), (20, 0, 7));
    assert_eq!(game.state.zone(Zone::hand(PlayerId(0))).len(), 9);
    assert_eq!(game.state.zone(Zone::hand(PlayerId(1))).len(), 0);
}

#[test]
fn a_deal_refuses_what_no_game_can_start_with() {
    let decks = vec![vec!["Forest".to_string(); 40]; 2];
    let refuse = |variant: SeatVariantNames| {
        let variant = [variant];
        let outside = Outside { variant: &variant, ..Outside::default() };
        deal_with(&decks, &outside, 1, false, None).err()
    };
    assert!(refuse(SeatVariantNames { emblems: vec!["Forest".into()], ..SeatVariantNames::default() }).is_some(), "a card is not an emblem");
    assert!(refuse(SeatVariantNames { emblems: vec!["No Such Emblem".into()], ..SeatVariantNames::default() }).is_some());
    assert!(refuse(SeatVariantNames { starting_hand_size: Some(u8::MAX), ..SeatVariantNames::default() }).is_some());
    assert!(refuse(SeatVariantNames { emblems: vec!["Chandra, Awakened Inferno emblem".into()], ..SeatVariantNames::default() }).is_none());
}
