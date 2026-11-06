//! Blessed Wine schedules its draw for the next turn, not another upkeep this turn.
use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};

#[test]
fn blessed_wine_schedules_the_next_turns_upkeep() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap();
    let deck = vec![card("Plains"); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    let you = PlayerId(0);
    game.state.update_turn(|t| { t.active = you; t.priority = you; t.set_step(Step::PrecombatMain); });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let land = game.state.push_obj(Obj::new(card("Plains"), you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, land);
    let wine = game.state.push_obj(Obj::new(card("Blessed Wine"), you, Zone::hand(you)));
    game.state.zone_push(Zone::hand(you), wine);
    game.state.add_mana(you, Mana::W, 2, land, None, ManaLasts::Step, false);
    game.state.see_statics();
    let mut cast = false;
    for _ in 0..30 {
        game.run();
        if cast && !game.state.delayed.is_empty() { break; }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == you && !cast => {
                let option = options.iter().find(|o| o.source == Some(wine) && o.kind == ActionKind::CastSpell).expect("Blessed Wine offered");
                cast = true;
                Answer::Act(Action::of(option))
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
