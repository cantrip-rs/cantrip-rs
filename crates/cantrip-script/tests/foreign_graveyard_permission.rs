//! Havengul Lich's resolved permission can cover another player's creature card (CR 601.2, 601.3).
//! It changes the casting zone, while normal timing and costs still apply.

use std::sync::OnceLock;

use cantrip_engine::*;

static BEAR: CardDef = CardDef {
    name: "Probe Borrowed Bear",
    cost: ManaCost { generic: 1, symbols: [0, 0, 0, 0, 1, 0], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static PERMISSION: CardDef = CardDef { name: "Probe Permission Source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static TWO_GREEN: CardDef = CardDef {
    name: "Probe Two Green",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_n(Mana::G, 2), "{T}: Add {G}{G}.")],
    ..CardDef::BLANK
};
static ONE_GREEN: CardDef = CardDef {
    name: "Probe One Green",
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..TWO_GREEN
};
static FILLER: CardDef = CardDef { name: "Probe Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&BEAR, &PERMISSION, &TWO_GREEN, &ONE_GREEN, &FILLER]))
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table().id_of(def), owner, zone));
    game.state.zone_push(zone, id);
    id
}

fn setup(recipient: Option<PlayerId>, enough_mana: bool) -> (Game, ObjectId, ObjectId) {
    let filler = table().id_of(&FILLER);
    let mut game = Game::new(table(), &Setup { decks: vec![vec![filler; 30]; 2], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let bear = put(&mut game, &BEAR, PlayerId(1), Zone::graveyard(PlayerId(1)));
    let source = put(&mut game, &PERMISSION, PlayerId(0), Zone::BATTLEFIELD);
    let bank = put(&mut game, if enough_mana { &TWO_GREEN } else { &ONE_GREEN }, PlayerId(0), Zone::BATTLEFIELD);
    if let Some(player) = recipient {
        game.state.permission_push(PlayPermission {
            what: Playable::Card(bear),
            player,
            grant: PlayGrant { expiry: Expiry::EndOfTurn, ..PlayGrant::CAST },
            by: Grant { object: source, ability: AbilityRef::spell(table().id_of(&PERMISSION)) },
            turn: 1,
        });
    }
    (game, bear, bank)
}

fn main_phase(game: &mut Game) -> Request {
    for _ in 0..100 {
        game.run();
        let request = game.pending().cloned().expect("the game continues");
        if request.player == PlayerId(0) && game.state.turn.step() == Step::PrecombatMain {
            return request;
        }
        game.answer(request.id, request.player, Answer::Pass).unwrap();
    }
    panic!("did not reach the first main phase")
}

#[test]
fn a_foreign_graveyard_cast_needs_the_right_permission_and_its_normal_mana() {
    for (recipient, enough_mana) in [(None, true), (Some(PlayerId(1)), true), (Some(PlayerId(0)), false)] {
        let (mut game, bear, _) = setup(recipient, enough_mana);
        let request = main_phase(&mut game);
        let RequestKind::Priority { options } = request.kind else { panic!("priority") };
        assert!(!options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(bear)));
    }

    let (mut game, bear, bank) = setup(Some(PlayerId(0)), true);
    let request = main_phase(&mut game);
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let cast = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(bear)).expect("the explicit permission offers the opponent's Bear");
    game.answer(request.id, request.player, Answer::Act(Action::of(cast))).unwrap();
    let mut entered = None;
    let mut saw_cost = false;
    for _ in 0..100 {
        game.run();
        if let Some(id) = game.state.battlefield().iter().copied().find(|&id| game.name(id) == BEAR.name) {
            entered = Some(id);
            break;
        }
        let request = game.pending().cloned().expect("the cast continues");
        let answer = match &request.kind {
            RequestKind::PayCosts { options, total, .. } => {
                assert_eq!(total.to_string(), "{1}{G}");
                saw_cost = true;
                let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(bank))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                    .expect("pay with the two green mana");
                Answer::Act(Action::of(option))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            other => panic!("unexpected request {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    let entered = entered.expect("the borrowed creature resolved");
    assert!(saw_cost);
    assert_eq!(game.state.obj(entered).owner, PlayerId(1));
    assert_eq!(game.state.obj(entered).controller, PlayerId(0));
    assert!(game.state.player(PlayerId(0)).pool.is_empty());
    assert!(game.history().all().iter().any(|r| matches!(r.event, Event::Cast(c) if c.player == PlayerId(0) && c.from == ZoneKind::Graveyard)));
    assert!(!game.state.permissions.iter().any(|p| game.state.permission_covers(p, entered)), "the old permission does not follow the new permanent");
}
