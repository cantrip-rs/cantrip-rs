//! A permission to play a card that lasts "until [a player's] next turn" (CR 611.2b) ends as that player's next turn begins, whoever
//! holds it: Memory Vessel's exiled cards are played by their owners until the Vessel's controller's turn comes round.

use std::sync::OnceLock;

use cantrip_engine::*;

static FILLER: CardDef = CardDef { name: "Probe Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static EXILED: CardDef = CardDef { name: "Probe Exiled", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static SOURCE: CardDef = CardDef { name: "Probe Source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&FILLER, &EXILED, &SOURCE]))
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table().id_of(def), owner, zone));
    game.state.zone_push(zone, id);
    id
}

/// Pass until `done` holds of the game, every request answered with a pass.
fn pass_until(game: &mut Game, done: impl Fn(&Game) -> bool) {
    for _ in 0..400 {
        game.run();
        if done(game) {
            return;
        }
        let request = game.pending().cloned().expect("the game continues");
        game.answer(request.id, request.player, Answer::Pass).unwrap();
    }
    panic!("did not get there");
}

#[test]
fn a_permission_held_by_the_other_player_lasts_until_the_named_players_next_turn_begins() {
    let filler = table().id_of(&FILLER);
    let mut game = Game::new(table(), &Setup { decks: vec![vec![filler; 30]; 2], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let source = put(&mut game, &SOURCE, PlayerId(0), Zone::BATTLEFIELD);
    // Player 0's card, played by player 0, until player 1's next turn: the holder is not the player the expiry names.
    let card = put(&mut game, &EXILED, PlayerId(0), Zone::exile(PlayerId(0)));
    game.state.permission_push(PlayPermission {
        what: Playable::Card(card),
        player: PlayerId(0),
        grant: PlayGrant { lands: true, expiry: Expiry::UntilNextTurnOf(PlayerId(1)), ..PlayGrant::CAST },
        by: Grant { object: source, ability: AbilityRef::spell(table().id_of(&SOURCE)) },
        turn: 1,
    });
    pass_until(&mut game, |g| g.state.turn.number == 1 && g.state.turn.step() == Step::End);
    assert_eq!(game.state.permissions.len(), 1, "the holder's own turn, and the turn it was given, do not end it");
    pass_until(&mut game, |g| g.state.turn.number == 2 && g.state.turn.step() == Step::Upkeep);
    assert!(game.state.permissions.is_empty(), "it ends as player 1's next turn begins");
}
