//! Actual Gix's Command resolution and selection rules; only its mana cost is waived to arrange the position.
//! Concede at the departing opponent's own sacrifice prompt, so replay must process the concession before
//! constructing that opponent's choice (CR 104.3a, 800.4a). No default answer may hide a stand-in prompt.
use cantrip_engine::*;
use std::sync::OnceLock;

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let cards = cantrip_cards::all();
        let gix = cards.iter().copied().find(|c| c.name == "Gix's Command").unwrap();
        let free: &'static CardDef = Box::leak(Box::new(CardDef { cost: ManaCost::ZERO, ..*gix }));
        CardTable::new(cards.into_iter().map(|c| if c.name == free.name { free } else { c }))
    })
}

fn departing_opponent_is_not_asked_through_a_stand_in(players: usize) {
    let table = table();
    let id = |name| table.find(name).unwrap();
    let mut decks = vec![vec![id("Plains"); 30]; players];
    decks[0][0] = id("Gix's Command");
    let mut game = Game::new(table, &Setup { decks, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    for seat in 1..players {
        for _ in 0..2 {
            let object = game.state.push_obj(Obj::new(id("Ornithopter"), PlayerId(seat as u8), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, object);
        }
    }
    let (mut cast, mut conceded, mut resolved) = (false, false, false);
    let mut asked = vec![0; players];
    for _ in 0..500 {
        game.run();
        let request = game.pending().cloned().expect("game must reach a question");
        if cast && game.state.stack.is_empty() && game.state.triggers.is_empty()
            && matches!(request.kind, RequestKind::Priority { .. }) {
            resolved = true;
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if !cast && request.player == PlayerId(0)
                && game.state.turn.step() == Step::PrecombatMain => {
                let option = options.iter().find(|o| o.kind == ActionKind::CastSpell
                    && o.source.is_some_and(|s| game.name(s) == "Gix's Command")).expect("Gix offered");
                cast = true;
                Answer::Act(Action::of(option))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseModes { .. } => Answer::Modes(vec![2, 3]),
            RequestKind::ChooseCards { purpose: Purpose::Sacrifice, .. } => {
                let seat = request.player.0 as usize;
                asked[seat] += 1;
                assert_ne!(seat, 0, "controller was asked to sacrifice for a departed opponent");
                assert_eq!(asked[seat], 1, "an opponent was asked again to choose for the departed player");
                if request.player == PlayerId(2) && !conceded {
                    game.concede(PlayerId(2));
                    conceded = true;
                    continue;
                }
                assert!(!game.state.player(request.player).lost, "departed player was asked again");
                Answer::Cards(vec![first_choices(&game, &request)[0]])
            }
            RequestKind::ChoosePlayer { .. } => panic!("departed opponent's sacrifice was routed to a stand-in"),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(cast && conceded && resolved);
    assert!(game.state.player(PlayerId(2)).lost);
    for seat in 1..players {
        let left = game.state.battlefield().iter().filter(|&&object| game.state.obj(object).controller == PlayerId(seat as u8)).count();
        assert_eq!(left, if seat == 2 { 0 } else { 1 }, "wrong sacrifice result for seat {seat}");
    }
}

#[test]
fn three_players_do_not_route_the_departed_opponents_choice_to_the_surviving_opponent() {
    departing_opponent_is_not_asked_through_a_stand_in(3);
}

#[test]
fn four_players_do_not_ask_the_controller_to_choose_a_stand_in() {
    departing_opponent_is_not_asked_through_a_stand_in(4);
}
