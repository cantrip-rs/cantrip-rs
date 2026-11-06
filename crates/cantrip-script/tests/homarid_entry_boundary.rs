//! Model the counter state carried by Dominion Saboteur before copied Homarid's entry effect.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::effects;

fn copied_entry(g: &mut dyn Rules, entry: &mut Entering) -> Result<(), Suspend> {
    entry.counters[CounterKind::Tide as usize] = i32::from(i16::MAX);
    effects::enters_with_a_counter::<{ CounterKind::Tide as usize }>(g, entry)?;
    // Upstream PR #2406 makes 32768 representable in the counter proposal and stored field.
    assert_eq!(entry.counters[CounterKind::Tide as usize], i32::from(i16::MAX) + 1);
    Ok(())
}

static PROBE: CardDef = CardDef { name: "Entry Boundary Probe", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), enters: Some(copied_entry), ..CardDef::BLANK };

#[test]
fn copied_homarid_entry_crosses_the_former_i16_counter_ceiling() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&PROBE]))));
    let island = table.find("Island").unwrap();
    let probe = table.find("Entry Boundary Probe").unwrap();
    let mut deck = vec![probe];
    deck.extend(vec![island; 29]);
    let mut game = Game::new(table, &Setup { decks: vec![deck, vec![island; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for _ in 0..1000 {
        game.run();
        if let Some(&id) = game.state.battlefield().iter().find(|&&id| game.name(id) == "Entry Boundary Probe") {
            assert_eq!(game.state.obj(id).counters(CounterKind::Tide), 32768);
            return;
        }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { options } => options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == "Entry Boundary Probe")).map_or(Answer::Pass, |o| Answer::Act(Action::of(o))),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected request {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("probe never entered");
}
