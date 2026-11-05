//! The Backdraft proposal exceeds u16 while staying inside the engine's damage domain.
use cantrip_engine::*;

#[test]
fn printed_backdraft_does_not_cap_the_sorcery_damage_at_u16() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap_or_else(|| panic!("missing {name}"));
    let mut deck = vec![card("Chain Lightning"), card("Backdraft")];
    deck.extend(std::iter::repeat(card("Island")).take(28));
    let mut game = Game::new(table, &Setup { decks: vec![deck, vec![card("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in std::iter::repeat("Furnace of Rath").take(8).chain(["Platinum Emperion", "Mox Ruby", "Mox Ruby", "Mox Pearl"]) {
        let id = game.state.push_obj(Obj::new(card(name), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    let emperor = game.state.push_obj(Obj::new(card("Platinum Emperion"), PlayerId(1), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, emperor);
    let mut cast_chain = false;
    let mut cast_backdraft = false;
    for _ in 0..1000 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("game ended before Backdraft resolved") };
        if cast_backdraft && game.state.stack.is_empty() && matches!(request.kind, RequestKind::Priority { .. }) { break; }
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.turn.step() == Step::PrecombatMain && game.state.stack.is_empty() => {
                let name = if !cast_chain { "Chain Lightning" } else { "Backdraft" };
                if let Some(option) = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)) {
                    let mut action = Action::of(option);
                    if !cast_chain { action.targets = vec![Target::Player(PlayerId(1))]; cast_chain = true; } else { cast_backdraft = true; }
                    Answer::Act(action)
                } else { Answer::Pass }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseReplacement { .. } => Answer::Index(0),
            RequestKind::ChooseTarget { candidates, .. } => Answer::Targets(vec![candidates[0]]),
            RequestKind::ChooseCards { cards, .. } => Answer::Cards(vec![cards[0]]),
            RequestKind::UnlessPays { options, .. } => Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::DeclineCost).expect("decline copy"))),
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).expect("mana payment")),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let option = if *mana_abilities && !remaining.is_zero() {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Mox Ruby" || game.name(s) == "Mox Pearl"))
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
    assert!(cast_chain && cast_backdraft);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Damage { source, amount: 98_304, .. } if game.name(source) == "Backdraft")));
}
