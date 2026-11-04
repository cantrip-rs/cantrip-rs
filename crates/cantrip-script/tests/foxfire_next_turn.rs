//! Foxfire binds its delayed draw to the next turn, not the next upkeep of this turn.
use cantrip_engine::*;
use cantrip_state::{Attacker, Frame, FrameKind};

#[test]
fn foxfire_schedules_the_next_turns_upkeep() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap();
    let deck = vec![card("Forest"); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    let you = PlayerId(0);
    let them = PlayerId(1);
    game.state.update_turn(|t| { t.active = you; t.priority = you; t.set_step(Step::DeclareAttackers); });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let bear = game.state.push_obj(Obj::new(card("Grizzly Bears"), you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, bear);
    game.state.set_attackers(vec![Attacker::new(bear, Target::Player(them), them, TypeSet::NONE)]);
    let fox = game.state.push_obj(Obj::new(card("Foxfire"), you, Zone::hand(you)));
    game.state.zone_push(Zone::hand(you), fox);
    game.state.add_mana(you, Mana::G, 3, bear, None, ManaLasts::Step, false);
    game.state.see_statics();
    let mut cast = false;
    for _ in 0..30 {
        game.run();
        if cast && !game.state.delayed.is_empty() { break; }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == you && !cast => {
                let option = options.iter().find(|o| o.source == Some(fox) && o.kind == ActionKind::CastSpell).expect("Foxfire offered");
                let mut action = Action::of(option);
                action.targets = vec![Target::Object(bear)];
                cast = true;
                Answer::Act(action)
            }
            _ => Answer::Pass,
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(game.state.delayed.len(), 1);
    assert!(matches!(game.state.delayed[0].when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}
