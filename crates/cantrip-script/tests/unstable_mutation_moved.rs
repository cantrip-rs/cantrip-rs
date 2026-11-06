//! An Aura moved in response to its upkeep trigger still refers to the creature named by the trigger.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_api::Game as Rules;

fn move_aura(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let aura = g.state().battlefield().iter().copied().find(|&id| g.chars(id).name == "Unstable Mutation").unwrap();
    let host = g.state().battlefield().iter().copied().find(|&id| g.chars(id).name == "Grizzly Bears").unwrap();
    g.attach(aura, host);
    Ok(())
}
static MOVER: CardDef = CardDef {
    name: "Probe Aura Mover", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{0}"), move_aura, "Move the Aura.")], ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MOVER])))
}

fn put(game: &mut Game, name: &str, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

#[test]
fn moving_aura_does_not_change_the_triggered_creature() {
    let land = table().find("Island").unwrap();
    let mut game = Game::new(table(), &Setup { decks: vec![vec![land; 30], vec![land; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    let you = PlayerId(0);
    let first = put(&mut game, "Ornithopter", you);
    let second = put(&mut game, "Grizzly Bears", you);
    let mover = put(&mut game, "Probe Aura Mover", you);
    let aura = put(&mut game, "Unstable Mutation", you);
    game.state.update_obj(aura, |o| o.attached_to = Some(first));
    let mut moved = false;
    let mut resolved = false;
    for _ in 0..200 {
        game.run();
        let request = game.pending().cloned().expect("game continues");
        if moved && game.state.stack.is_empty() {
            resolved = true;
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if !moved && !game.state.stack.is_empty() => {
                let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(mover)).unwrap();
                moved = true;
                Answer::Act(Action::of(option))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCards { .. } => Answer::Cards(first_choices(&game, &request)),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(resolved, "the upkeep trigger resolved");
    assert_eq!(game.state.obj(first).counters(CounterKind::MinusOne), 1);
    assert_eq!(game.state.obj(second).counters(CounterKind::MinusOne), 0);
}
