//! The host can ask whether an opponent's creature could attack the active player.
//! Activity on the active player's current turn is not activity on their last turn.
use cantrip_engine::*;

#[test]
fn current_turn_spell_does_not_count_as_last_turn() {
    let cards = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let pearl = cards.find("Mox Pearl").unwrap();
    let filler = cards.find("Island").unwrap();
    let mut first = vec![pearl];
    first.extend([filler; 29]);
    let mut game = Game::new(cards, &Setup { decks: vec![first, vec![filler; 30]], shuffle: false, opening_chooser: None, ..Default::default() });
    let arboria = game.state.push_obj(Obj::new(cards.find("Arboria").unwrap(), PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, arboria);
    let creature = game.state.push_obj(Obj::new(cards.find("Memnite").unwrap(), PlayerId(1), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, creature);
    game.state.see_statics();
    let mut cast = false;
    for _ in 0..100 {
        game.run();
        let request = game.pending().cloned().expect("question");
        if cast && game.state.stack.is_empty() {
            assert_eq!(game.state.turn.number, 1);
            assert!(!game.attack_rules_allow(creature), "the current turn's spell must not expose its caster");
            return;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                if !cast && request.player == PlayerId(0) {
                    if let Some(option) = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|id| game.state.obj(id).card == pearl)) {
                        cast = true;
                        Answer::Act(Action::of(option))
                    } else { Answer::Pass }
                } else { Answer::Pass }
            }
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("spell was not cast");
}
