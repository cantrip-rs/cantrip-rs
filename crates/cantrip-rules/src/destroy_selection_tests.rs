//! A direct destruction choice asks the same rules permission as the verb, separately from targeting.
use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static SOFT: CardDef = CardDef {
    name: "Destroyable test creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
static HARD: CardDef = CardDef {
    name: "Indestructible test creature",
    keywords: Keywords::of(&[Keyword::Indestructible]),
    ..SOFT
};
fn cx(game: &mut Game) -> Cx<'_> {
    Cx {
        g: game,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: Vec::new(),
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn board() -> (Game, ObjectId, ObjectId) {
    let table = Box::leak(Box::new(CardTable::new([&SOFT, &HARD])));
    let mut game = Game::new(
        table,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    );
    let mut put = |card| {
        let id = game
            .state
            .push_obj(Obj::new(table.id_of(card), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let soft = put(&SOFT);
    let hard = put(&HARD);
    (game, soft, hard)
}
#[test]
fn indestructible_is_a_destruction_permission_not_a_targeting_restriction() {
    let (mut game, soft, hard) = board();
    assert!(game.can_destroy(soft));
    assert!(!game.can_destroy(hard));
    assert!(game.can_target(&Asker::new(soft, THEM), Target::Object(hard)));
    let moved = ops::destroy_together(&mut cx(&mut game), &[soft, hard], ZoneReason::Destroy);
    assert_eq!(moved.len(), 1);
    assert!(game.is_on_battlefield(hard));
    assert!(
        !game.can_destroy(soft),
        "the old identity left the battlefield"
    );
    assert!(
        !game.can_destroy(moved[0]),
        "a graveyard object is not a destruction choice"
    );
}
#[test]
fn destruction_permission_reads_the_current_layers() {
    let (mut game, soft, hard) = board();
    cantrip_api::Game::add_effect(
        &mut cx(&mut game),
        hard,
        soft,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible])),
        Duration::EndOfTurn,
    );
    assert!(!game.can_destroy(soft));
    assert!(
        ops::destroy_together(&mut cx(&mut game), &[soft], ZoneReason::DestroyNoRegen).is_empty()
    );
    cantrip_api::Game::add_effect(
        &mut cx(&mut game),
        hard,
        soft,
        EffectKind::LoseAllAbilities,
        Duration::EndOfTurn,
    );
    assert!(
        game.can_destroy(soft),
        "losing the granted ability makes destruction possible again"
    );
    assert_eq!(
        ops::destroy_together(&mut cx(&mut game), &[soft], ZoneReason::DestroyNoRegen).len(),
        1
    );
}
