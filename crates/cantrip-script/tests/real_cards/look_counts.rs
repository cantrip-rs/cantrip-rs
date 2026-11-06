//! A look at more cards than eight bits count, which the event cap (RX-1) lets a game reach: Pillage the Bog with 128
//! lands looks at the top 256 cards, every one of which may be taken, and still puts one of them into its controller's hand
//! (`look_at_top`: the bound of one is kept, never cut to the 256 that qualify wrapped to none).

use cantrip_engine::*;

#[test]
fn pillage_the_bog_with_128_lands_looks_at_256_cards_and_takes_one() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name: &str| table.find(name).unwrap_or_else(|| panic!("no card {name}"));
    let mut deck = vec![card("Pillage the Bog")];
    deck.extend(vec![card("Island"); 299]);
    let mut game = Game::new(table, &Setup { decks: vec![deck, vec![card("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in ["Swamp", "Forest"] {
        for _ in 0..64 {
            let id = game.state.push_obj(Obj::new(card(name), PlayerId(0), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id);
        }
    }
    let hand_before = 7;
    let mut looked = None;
    for _ in 0..500 {
        game.run();
        let request = game.pending().cloned().expect("a question");
        if looked.is_some() && game.state.stack.is_empty() {
            break;
        }
        let main = request.player == PlayerId(0) && game.state.turn.step() == Step::PrecombatMain && game.state.stack.is_empty();
        let answer = match &request.kind {
            RequestKind::Priority { options } if main && looked.is_none() => options
                .iter()
                .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|c| game.name(c) == "Pillage the Bog"))
                .map_or(Answer::Pass, |o| Answer::Act(Action::of(o))),
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let land = options.iter().find(|o| o.kind == ActionKind::ActivateAbility);
                super::pay(options, remaining, *mana_abilities, land)
            }
            RequestKind::DistributeCards { cards, destinations, .. } => {
                looked = Some((cards.len(), destinations[0].min, destinations[0].max));
                let taken = vec![cards[0]];
                Answer::Placements { placements: vec![taken, cards[1..].to_vec()], shuffle: false }
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(looked, Some((256, 1, 1)), "256 cards looked at, one to be taken");
    assert_eq!(game.state.zone(Zone::hand(PlayerId(0))).len(), hand_before, "Pillage the Bog left the hand, one card came in");
}
