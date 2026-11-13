//! The two-target set must be controlled by one opponent, including during partial selection.
use cantrip_api::select::{first_targets, judge_targets, targets_together};
use cantrip_engine::*;

#[test]
fn target_set_keeps_both_blockers_with_the_same_controller() {
    let cards = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let land = cards.find("Sorrow's Path").unwrap();
    let filler = cards.find("Island").unwrap();
    let mut game = Game::new(cards, &Setup { decks: vec![vec![filler; 20]; 3], ..Default::default() });
    let add = |g: &mut Game, player| {
        let id = g.state.push_obj(Obj::new(cards.find("Memnite").unwrap(), PlayerId(player), Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id);
        Target::Object(id)
    };
    let a = add(&mut game, 1);
    let b = add(&mut game, 1);
    let c = add(&mut game, 2);
    let AbilityDef::Activated(ability) = &cards.get(land).abilities[0] else { panic!("activated ability") };
    let part = ability.targets[0].together.expect("same controller rule");
    assert_eq!((ability.targets[0].min, ability.targets[0].max), (2, 2));
    assert_eq!(part.words, "with the same controller");
    let choices = [a, b, c];
    let verdict = judge_targets(&game, &[part], PlayerId(0), ObjectId::NONE, &choices, &[a]);
    assert!(verdict.addable.get(1));
    assert!(!verdict.addable.get(2));
    assert!(targets_together(&game, &[part], PlayerId(0), ObjectId::NONE, &[a, b]));
    assert!(!targets_together(&game, &[part], PlayerId(0), ObjectId::NONE, &[a, c]));
    assert!(first_targets(&game, Some(&part), PlayerId(0), ObjectId::NONE, &[a, c], 2).is_none());
    assert_eq!(first_targets(&game, Some(&part), PlayerId(0), ObjectId::NONE, &choices, 2), Some(vec![a, b]));
}

#[test]
fn target_filter_accepts_only_opposing_blocking_creatures() {
    let cards = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let island = cards.find("Island").unwrap();
    let mut game = Game::new(cards, &Setup { decks: vec![vec![island; 20]; 3], ..Default::default() });
    let mut add = |name: &str, controller| {
        let id = game.state.push_obj(Obj::new(cards.find(name).unwrap(), PlayerId(controller), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let source = add("Sorrow's Path", 0);
    let opponent_blocker = add("Memnite", 1);
    let opponent_nonblocker = add("Ornithopter", 1);
    let own_blocker = add("Memnite", 0);
    let opponent_land = add("Island", 1);
    game.state.set_blockers(vec![opponent_blocker, own_blocker]);
    let AbilityDef::Activated(ability) = &cards.get(cards.find("Sorrow's Path").unwrap()).abilities[0] else { panic!("activated ability") };
    let filter = ability.targets[0].filter;
    let asker = Asker::new(source, PlayerId(0));
    assert!(filter(&game, &asker, Target::Object(opponent_blocker)));
    assert!(!filter(&game, &asker, Target::Object(opponent_nonblocker)));
    assert!(!filter(&game, &asker, Target::Object(own_blocker)));
    assert!(!filter(&game, &asker, Target::Object(opponent_land)));
}
