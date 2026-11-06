//! Forced position built solely from printed definitions; all amounts fit the engine damage domain.
use cantrip_engine::*;
#[test]
fn simulacrum_preserves_large_in_range_damage() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap_or_else(|| panic!("missing {name}"));
    let mut first = vec![card("Simulacrum")];
    first.extend(std::iter::repeat(card("Island")).take(29));
    let mut game = Game::new(table, &Setup { decks: vec![first, vec![card("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in std::iter::repeat("Furnace of Rath").take(6)
        .chain(["Platinum Angel", "Ornithopter", "Pyrite Spellbomb", "Mox Ruby", "Mox Ruby", "Mox Jet", "Mox Jet"]) {
        let id = game.state.push_obj(Obj::new(card(name), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    let mut blasts = 0;
    let mut cast = false;
    for _ in 0..1000 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("game ended") };
        if cast && game.state.stack.is_empty() && matches!(request.kind, RequestKind::Priority { .. }) { break; }
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.turn.step() == Step::PrecombatMain && game.state.stack.is_empty() => {
                let name = if blasts < 1 { "Pyrite Spellbomb" } else { "Simulacrum" };
                let kind = if blasts < 1 { ActionKind::ActivateAbility } else { ActionKind::CastSpell };
                if let Some(option) = options.iter().find(|o| o.kind == kind && o.index == 0 && o.source.is_some_and(|s| game.name(s) == name)) {
                    let mut action = Action::of(option);
                    if name == "Pyrite Spellbomb" { action.targets = vec![Target::Player(PlayerId(0))]; blasts += 1; }
                    else { let target = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Ornithopter").unwrap(); action.targets = vec![Target::Object(target)]; cast = true; }
                    Answer::Act(action)
                } else { Answer::Pass }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseReplacement { .. } => Answer::Index(0),
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { source, options, remaining, mana_abilities, .. } => {
                let name = if game.name(*source) == "Pyrite Spellbomb" { "Mox Ruby" } else { "Mox Jet" };
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
    assert_eq!(blasts, 1);
    assert_eq!(game.state.player(PlayerId(0)).life, 20);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Damage { source, amount: 8192, .. } if game.name(source) == "Simulacrum")));
    assert!(cast);
}
