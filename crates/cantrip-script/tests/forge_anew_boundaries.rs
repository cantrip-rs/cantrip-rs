//! Forge Anew's target, timing and alternate-cost boundaries.
use cantrip_engine::*;
use cantrip_state::Obj;

#[test]
fn target_and_equip_permissions_respect_type_controller_and_turn() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let you = PlayerId(0);
    let opponent = PlayerId(1);
    let island = table.find("Island").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![island; 30]; 2], ..Default::default() });
    let forge_def = table.find("Forge Anew").unwrap();
    let axe_def = table.find("Bloodforged Battle-Axe").unwrap();
    let mox_def = table.find("Mox Pearl").unwrap();
    let source = game.state.push_obj(Obj::new(forge_def, you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let axe = game.state.push_obj(Obj::new(axe_def, you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, axe);
    let mox = game.state.push_obj(Obj::new(mox_def, you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, mox);
    let grave = Zone::graveyard(you);
    let grave_axe = game.state.push_obj(Obj::new(axe_def, you, grave));
    game.state.zone_push(grave, grave_axe);
    let grave_mox = game.state.push_obj(Obj::new(mox_def, you, grave));
    game.state.zone_push(grave, grave_mox);
    let other_grave = Zone::graveyard(opponent);
    let other_axe = game.state.push_obj(Obj::new(axe_def, opponent, other_grave));
    game.state.zone_push(other_grave, other_axe);
    let def = table.get(forge_def);
    let AbilityDef::Triggered(trigger) = def.abilities[0] else { panic!("expected trigger") };
    let filter = trigger.targets[0].filter;
    let asker = Asker::new(source, you);
    assert!(filter(&game, &asker, Target::Object(grave_axe)));
    assert!(!filter(&game, &asker, Target::Object(grave_mox)));
    assert!(!filter(&game, &asker, Target::Object(other_axe)));
    assert!(!filter(&game, &asker, Target::Object(axe)));
    let timing = def.instant_activation[0];
    let alternative = def.ability_alternatives[0];
    let equip_ref = Query::chars(&game, axe).abilities().find(|a| Query::activated_ability(&game, *a).is_some_and(|d| d.equip)).unwrap();
    let equip_index = Query::chars(&game, axe).abilities().position(|a| a == equip_ref).unwrap() as u8;
    let equip = Query::activated_ability(&game, equip_ref).unwrap();
    let mana_ref = Query::chars(&game, mox).abilities().find(|a| Query::activated_ability(&game, *a).is_some()).unwrap();
    let mana = Query::activated_ability(&game, mana_ref).unwrap();
    game.state.update_turn(|turn| turn.active = you);
    assert!(timing(&game, &asker, axe, equip_index, you));
    assert!(!timing(&game, &asker, axe, equip_index, opponent));
    assert!(!timing(&game, &asker, mox, 0, you));
    assert!(alternative(&game, &asker, you, axe, &equip).is_some());
    assert!(alternative(&game, &asker, opponent, axe, &equip).is_none());
    assert!(alternative(&game, &asker, you, mox, &mana).is_none());
    game.state.update_turn(|turn| turn.active = opponent);
    assert!(!timing(&game, &asker, axe, equip_index, you));
    assert!(!timing(&game, &asker, axe, equip_index, opponent));
    assert!(alternative(&game, &asker, you, axe, &equip).is_none());
    assert!(alternative(&game, &asker, opponent, axe, &equip).is_none());
}
