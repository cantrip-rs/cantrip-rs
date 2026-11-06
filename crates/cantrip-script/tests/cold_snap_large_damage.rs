//! Cold Snap counts more snow lands than fit in u16 before dealing damage (CR 608.2h).
use cantrip_engine::*;

#[test]
fn cold_snap_deals_damage_for_every_snow_land_past_u16() {
    let cards = cantrip_cards::all();
    let cold = *cards.iter().find(|c| c.name == "Cold Snap").unwrap();
    let snow = *cards.iter().find(|c| c.name == "Snow-Covered Forest").unwrap();
    let filler = *cards.iter().find(|c| c.name == "Island").unwrap();
    let table = Box::leak(Box::new(CardTable::new([cold, snow, filler])));
    let deck = vec![table.id_of(filler); 30];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.state.update_player(PlayerId(0), |p| p.life = 100_000);
    let source = game.state.push_obj(Obj::new(table.id_of(cold), PlayerId(1), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    for _ in 0..65_536 {
        let id = game.state.push_obj(Obj::new(table.id_of(snow), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    for _ in 0..200 {
        game.run();
        if game.state.player(PlayerId(0)).life < 100_000 { break; }
        let request = game.pending().cloned().expect("game continues");
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(game.state.player(PlayerId(0)).life, 34_464, "Cold Snap must deal all 65,536 damage");
}
