//! A skipped combat followed by an extra beginning phase (CR 506.8e).
use cantrip_api::{Asker, Cause, Event, Record};
use cantrip_engine::*;

#[test]
fn skipped_combat_does_not_reopen_casting_in_extra_beginning() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap();
    let deck = vec![card("Island"); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    let player = PlayerId(0);
    let spell = game.state.push_obj(Obj::new(card("Rapid Fire"), player, Zone::hand(player)));
    let condition = table.get(card("Rapid Fire")).cast_only_if.unwrap();
    game.state.update_turn(|t| { t.number = 1; t.active = player; t.set_step(Step::Upkeep); });
    let asker = Asker::spell(spell, player);
    assert!(condition(&game, &asker), "the first upkeep is before combat");
    // Fatespinner skips combat; after the postcombat main, Sphinx of the Second Sun adds a beginning phase.
    for step in [Step::PrecombatMain, Step::PostcombatMain, Step::Upkeep] {
        game.log.push(Record { serial: game.log.len() as u32, turn: 1, step, active: player, cause: Cause::Turn, event: Event::StepBegan { step } });
    }
    game.state.update_turn(|t| { t.phase_additional = true; t.set_step(Step::Upkeep); });
    assert!(!condition(&game, &asker), "the additional upkeep is after the skipped combat");
    game.state.update_turn(|t| t.set_step(Step::Draw));
    assert!(!condition(&game, &asker), "the additional draw step is after the skipped combat");
}
