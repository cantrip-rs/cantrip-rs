//! The mana ability schedules its draw for the next turn's upkeep.
use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};

#[test]
fn mana_ability_schedules_the_next_turns_upkeep() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap();
    let deck = vec![card("Island"); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    let you = PlayerId(0);
    game.state.update_turn(|t| { t.active = you; t.priority = you; t.set_step(Step::PrecombatMain); });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let sextant = game.state.push_obj(Obj::new(card("Barbed Sextant"), you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, sextant);
    game.state.add_mana(you, Mana::U, 1, sextant, None, ManaLasts::Step, false);
    game.state.see_statics();
    let mut activated = false;
    for _ in 0..30 {
        game.run();
        if activated && !game.state.delayed.is_empty() { break; }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == you && !activated => {
                let option = options.iter().find(|o| o.source == Some(sextant)).expect("Sextant offered");
                activated = true;
                let mut action = Action::of(option);
                action.color = Some(Color::Green);
                Answer::Act(action)
            }
            RequestKind::ChooseColors { .. } => Answer::Colors(vec![Color::Green]),
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .expect("the funded payment is available");
                Answer::Act(Action::of(pay))
            }
            RequestKind::ChooseCostPayment { choices, .. } => {
                let index = ["Pay mana", "Tap Barbed Sextant", "Sacrifice Barbed Sextant"].iter()
                    .find_map(|label| choices.iter().position(|choice| choice.label.starts_with(label)))
                    .expect("the Sextant's payment component is available");
                Answer::CostPayment(index)
            }
            _ => Answer::Pass,
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(game.state.delayed.len(), 1);
    assert!(matches!(game.state.delayed[0].when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}
