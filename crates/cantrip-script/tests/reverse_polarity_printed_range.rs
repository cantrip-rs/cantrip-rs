//! A forced position using printed cards, with damage totals inside the existing domain.
use cantrip_engine::*;

#[test]
fn reverse_polarity_counts_two_large_artifact_damage_events() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap_or_else(|| panic!("missing {name}"));
    let mut deck = vec![card("Reverse Polarity")];
    deck.extend(std::iter::repeat(card("Island")).take(29));
    let mut game = Game::new(table, &Setup { decks: vec![deck, vec![card("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in std::iter::repeat("Furnace of Rath").take(16).chain(["Platinum Angel", "Pyrite Spellbomb", "Pyrite Spellbomb", "Mox Ruby", "Mox Ruby", "Mox Pearl", "Mox Pearl"]) {
        let id = game.state.push_obj(Obj::new(card(name), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    let mut blasts = 0;
    let mut cast = false;
    for _ in 0..1000 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("game ended before Reverse Polarity resolved") };
        if cast && game.state.stack.is_empty() && matches!(request.kind, RequestKind::Priority { .. }) { break; }
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.turn.step() == Step::PrecombatMain && game.state.stack.is_empty() => {
                let name = if blasts < 2 { "Pyrite Spellbomb" } else { "Reverse Polarity" };
                let kind = if blasts < 2 { ActionKind::ActivateAbility } else { ActionKind::CastSpell };
                if let Some(option) = options.iter().find(|o| o.kind == kind && o.index == 0 && o.source.is_some_and(|s| game.name(s) == name)) {
                    let mut action = Action::of(option);
                    if blasts < 2 { action.targets = vec![Target::Player(PlayerId(0))]; blasts += 1; } else { cast = true; }
                    Answer::Act(action)
                } else { Answer::Pass }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseReplacement { .. } => Answer::Index(0),
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { source, options, remaining, mana_abilities, .. } => {
                let name = if game.name(*source) == "Pyrite Spellbomb" { "Mox Ruby" } else { "Mox Pearl" };
                let option = if *mana_abilities && !remaining.is_zero() {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == name))
                } else {
                    options.iter().find(|o| o.kind == ActionKind::AssignMana)
                        .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                }.expect("mana source or payment");
                Answer::Act(Action::of(option))
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(blasts, 2);
    assert!(cast);
    assert_eq!(game.state.player(PlayerId(0)).life, 262164);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::LifeChange { player: PlayerId(0), delta: 524288 })));
}
