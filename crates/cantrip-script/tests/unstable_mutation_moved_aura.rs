//! The upkeep trigger remembers the creature it identified even if its Aura moves (CR 608.2k).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_api::Game as Rules;

fn move_aura(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let ids = g.state().battlefield().to_vec();
    let aura = ids.iter().copied().find(|&id| g.card(g.obj(id).card).name == "Unstable Mutation").unwrap();
    let old = g.obj(aura).attached_to.unwrap();
    let next = ids.iter().copied().find(|&id| id != old && g.card(g.obj(id).card).name == "Wall of Earth").unwrap();
    assert!(g.attach(aura, next));
    Ok(())
}
static MOVE: CardDef = CardDef { name: "Probe Move Aura", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], move_aura), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MOVE])))
}

fn moved_aura(name: &str, counter: CounterKind) {
    let island = table().find("Island").unwrap();
    let mut deck = vec![table().find("Probe Move Aura").unwrap()];
    deck.extend(vec![island; 29]);
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![island; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    let mut put = |name: &str| {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let a = put("Wall of Earth");
    let b = put("Wall of Earth");
    let aura = put(name);
    game.state.update_obj(aura, |o| o.attached_to = Some(a));
    let mut moved = false;
    for _ in 0..200 {
        game.run();
        if moved && game.state.stack.is_empty() { break; }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { options } if !moved && request.player == PlayerId(0) && game.state.turn.step() == Step::Upkeep && !game.state.stack.is_empty() => {
                let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|id| game.name(id) == "Probe Move Aura")).expect("move spell offered");
                moved = true;
                Answer::Act(Action::of(option))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(moved, "upkeep trigger was not seen");
    assert_eq!(game.state.obj(a).counters(counter), 1, "the original creature gets the counter");
    assert_eq!(game.state.obj(b).counters(counter), 0, "the new enchanted creature does not");
}

#[test]
fn unstable_mutation_remembers_original_creature() { moved_aura("Unstable Mutation", CounterKind::MinusOne); }
