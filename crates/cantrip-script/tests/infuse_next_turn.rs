//! Infuse schedules a draw on the next turn's upkeep, not the next upkeep of this turn.
use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};

#[test]
fn infuse_schedules_the_next_turns_upkeep() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap();
    let deck = vec![card("Island"); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    let you = PlayerId(0);
    game.state.update_turn(|t| { t.active = you; t.priority = you; t.set_step(Step::PrecombatMain); });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let land = game.state.push_obj(Obj::new(card("Island"), you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, land);
    let infuse = game.state.push_obj(Obj::new(card("Infuse"), you, Zone::hand(you)));
    game.state.zone_push(Zone::hand(you), infuse);
    game.state.add_mana(you, Mana::U, 3, land, None, ManaLasts::Step, false);
    game.state.see_statics();
    let mut cast = false;
    for _ in 0..30 {
        game.run();
        if cast && !game.state.delayed.is_empty() { break; }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == you && !cast => {
                let option = options.iter().find(|o| o.source == Some(infuse) && o.kind == ActionKind::CastSpell).expect("Infuse offered");
                let mut action = Action::of(option);
                action.targets = vec![Target::Object(land)];
                cast = true;
                Answer::Act(action)
            }
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .expect("the funded payment is available");
                Answer::Act(Action::of(pay))
            }
            _ => Answer::Pass,
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(game.state.delayed.len(), 1);
    assert!(matches!(game.state.delayed[0].when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}
