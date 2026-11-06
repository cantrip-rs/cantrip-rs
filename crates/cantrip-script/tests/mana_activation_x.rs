//! A printed mana ability's X is local to the activation, including source sacrifice and wire-sized color questions.
use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};

static FILLER: CardDef = CardDef { name: "Mana activation filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

#[test]
fn wizards_rockets_produces_all_256_mana_after_sacrifice_without_replacing_its_casting_x() {
    let rockets = cantrip_cards::all().into_iter().find(|card| card.name == "Wizard's Rockets").unwrap();
    let table = Box::leak(Box::new(CardTable::new([&FILLER, rockets])));
    let deck = vec![table.id_of(&FILLER); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    game.verify = true;
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    let you = PlayerId(0);
    game.state.update_turn(|t| { t.active = you; t.priority = you; t.set_step(Step::PrecombatMain); });
    game.state.update_player(you, |p| p.turn_began = 1);
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let source = game.state.push_obj(Obj::new(table.id_of(rockets), you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    game.state.update_obj(source, |o| o.x = 13);
    game.state.add_mana(you, Mana::C, 256, source, None, ManaLasts::Step, false);
    game.state.see_statics();
    game.run();
    let request = game.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &request.kind else { panic!("{request:?}") };
    let action = Action::of(options.iter().find(|o| o.source == Some(source)).unwrap());
    game.answer(request.id, you, Answer::Act(action)).unwrap();
    game.run();
    let request = game.pending().cloned().unwrap();
    assert!(matches!(request.kind, RequestKind::PayCosts { announcing: true, .. }));
    game.answer(request.id, you, Answer::Announce(256)).unwrap();
    game.run();
    // Finish the activation payment explicitly, paying its mana before its sacrifice.
    for _ in 0..600 {
        let request = game.pending().cloned().unwrap();
        let answer = match &request.kind {
            RequestKind::ChooseColors { .. } => break,
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .expect("the funded mana payment is available");
                Answer::Act(Action::of(pay))
            }
            RequestKind::ChooseCostPayment { choices, .. } => {
                let index = ["Pay mana", "Sacrifice Wizard's Rockets"].iter()
                    .find_map(|label| choices.iter().position(|choice| choice.label.starts_with(label)))
                    .expect("the Rockets' payment component is available");
                Answer::CostPayment(index)
            }
            other => panic!("unexpected Rockets payment question: {other:?}"),
        };
        game.answer(request.id, you, answer).unwrap();
        game.run();
    }
    for (count, color) in [(255, Color::Green), (1, Color::Red)] {
        let request = game.pending().cloned().unwrap();
        assert!(matches!(request.kind, RequestKind::ChooseColors { count: n, .. } if n == count), "{request:?}");
        game.answer(request.id, you, Answer::Colors(vec![color; count as usize])).unwrap();
        game.run();
    }
    assert_eq!(game.state.player(you).pool.get(Mana::G), 255);
    assert_eq!(game.state.player(you).pool.get(Mana::R), 1);
    assert_eq!(game.state.player(you).pool.get(Mana::C), 0);
    assert_eq!(game.state.obj(source).x, 13);
    let moved = game.state.obj(source).became.expect("sacrificed to activate");
    assert_eq!(game.state.obj(moved).zone, Zone::graveyard(you));
}
