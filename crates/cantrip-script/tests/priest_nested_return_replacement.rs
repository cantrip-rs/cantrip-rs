//! Printed Priest, Mycosynth Lattice and Touch the Spirit Realm; the auxiliary land
//! supplies Lotus Vale's entry replacement without adding the card to the registry.
use cantrip_api::Game as Rules;
use cantrip_engine::*;

static ASK: Question = Question::new("Sacrifice lands", "Sacrifice two untapped lands?", "", "Sacrifice", "Don't sacrifice");

fn vale_entry(g: &mut dyn Rules, entering: &mut Entering) -> Result<(), Suspend> {
    let player = entering.controller;
    let lands: Vec<_> = g.state().battlefield().iter().copied().filter(|&id| {
        g.controller(id) == player && !g.obj(id).tapped && g.chars(id).is(CardType::Land) && g.can_sacrifice(player, id)
    }).collect();
    if lands.len() < 2 || g.ask(player, RequestKind::May { source: entering.object, purpose: Purpose::Ask(&ASK) })? != Answer::Yes {
        entering.instead = Some(Zone::graveyard(g.obj(entering.object).owner));
    } else {
        let sacrificed = cantrip_mechanics::sacrifice_one_of(g, player, entering.object, 2, lands)?;
        assert_eq!(sacrificed.len(), 2);
    }
    Ok(())
}

static VALE: CardDef = CardDef {
    name: "Lotus Vale entry fixture", oracle: true, types: TypeSet::of(&[CardType::Land]),
    enters: Some(vale_entry), ..CardDef::BLANK
};

#[test]
fn returned_vale_sacrifices_lands_before_priest_reads_the_paid_touch() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&VALE]))));
    let island = table.find("Island").unwrap();
    let deck = vec![island; 25];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let you = PlayerId(0);
    let add = |game: &mut Game, name: &str, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.find(name).unwrap(), you, zone));
        game.state.zone_push(zone, id);
        id
    };
    let priest = add(&mut game, "Priest of Yawgmoth", Zone::BATTLEFIELD);
    let _lattice = add(&mut game, "Mycosynth Lattice", Zone::BATTLEFIELD);
    game.state.see_statics();
    let land_a = add(&mut game, "Island", Zone::BATTLEFIELD);
    let land_b = add(&mut game, "Island", Zone::BATTLEFIELD);
    let vale = add(&mut game, "Lotus Vale entry fixture", Zone::BATTLEFIELD);
    for _ in 0..3 { add(&mut game, "Mox Pearl", Zone::BATTLEFIELD); }
    let touch = add(&mut game, "Touch the Spirit Realm", Zone::hand(you));
    let (mut cast, mut activated) = (false, false);
    let mut paid_touch = None;
    for _ in 0..500 {
        game.run();
        let request = game.pending().cloned().expect("the game is still running");
        if activated && matches!(request.kind, RequestKind::Priority { .. }) {
            let paid_touch = paid_touch.expect("the Touch permanent was selected");
            assert_eq!(game.state.player(you).pool.0[Mana::B as usize], 3, "Touch's mana value, not the returned land's sacrifice");
            let sacrifices: Vec<_> = game.log.iter().filter_map(|r| match r.event {
                Event::ZoneChange { old, reason: ZoneReason::Sacrifice, .. } if [paid_touch, land_a, land_b].contains(&old) => Some(old),
                _ => None,
            }).collect();
            assert_eq!(sacrifices.first(), Some(&paid_touch), "Touch is sacrificed as Priest's cost");
            assert_eq!(sacrifices.len(), 3, "the returning Vale's replacement sacrifices two lands immediately");
            assert!(sacrifices[1..].contains(&land_a) && sacrifices[1..].contains(&land_b));
            assert!(game.state.battlefield().iter().any(|&id| game.name(id) == "Lotus Vale entry fixture"));
            return;
        }
        let answer = match &request.kind {
            RequestKind::Mulligan { .. } => Answer::No,
            RequestKind::Priority { options } if request.player == you && !cast => {
                if let Some(option) = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(touch)) {
                    cast = true; Answer::Act(Action::of(option))
                } else { Answer::Pass }
            }
            RequestKind::Priority { options } if request.player == you && cast && game.state.stack.is_empty()
                && game.state.obj(vale).became.is_some_and(|id| game.state.obj(id).zone.is(ZoneKind::Exile)) => {
                if let Some(option) = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(priest)) {
                    paid_touch = game.state.battlefield().iter().copied().find(|&id| game.name(id) == "Touch the Spirit Realm");
                    activated = true; Answer::Act(Action::of(option))
                } else { Answer::Pass }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, mana_abilities: true, .. }
                if options.iter().any(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|id| game.name(id) == "Mox Pearl")) => {
                Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|id| game.name(id) == "Mox Pearl")).unwrap()))
            }
            RequestKind::PayCosts { options, .. } => {
                cantrip_engine::pay::auto_pay(&game, &request, &mut cantrip_engine::pay::PayPlan::default()).unwrap()
                    .unwrap_or_else(|| Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("close mana window"))))
            }
            RequestKind::ChooseTarget { candidates, .. } => {
                assert!(candidates.contains(&Target::Object(vale)));
                Answer::Targets(vec![Target::Object(vale)])
            }
            RequestKind::ChooseCostPayment { choices, .. } => {
                let tap = choices.iter().position(|c| c.kind == CostPaymentKind::Tap).unwrap_or(0);
                Answer::CostPayment(tap)
            }
            RequestKind::ChooseCards { cards, .. } if paid_touch.is_some_and(|id| cards.contains(&id)) => Answer::Cards(vec![paid_touch.unwrap()]),
            RequestKind::ChooseCards { cards, .. } => Answer::Cards(cards.iter().copied().filter(|id| *id == land_a || *id == land_b).collect()),
            RequestKind::May { .. } => Answer::Yes,
            other => panic!("unexpected question: {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("Priest never completed its activation");
}
