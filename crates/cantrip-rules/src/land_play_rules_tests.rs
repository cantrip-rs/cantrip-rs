//! A static ability that forbids some land plays (`CardDef::land_play_rules`, CR 101.2, 305.1), over test cards: it holds
//! against every way a land may be played - a rule for the graveyard (Crucible of Worlds' kind), a way a permanent gives
//! (Serra Paragon's kind), a permission an effect gave, and an effect that plays a land as it resolves (CR 608.2g) - and a
//! land played as the back face of a modal double-faced card is judged as that face (CR 712.12). A permanent that has lost
//! its abilities forbids nothing (CR 613.1f). The scenarios show the rest with Tomik, Distinguished Advokist itself.

use crate::actions::{could_play_land_during_resolution, priority_options, stack_item};
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "Your opponents can't play land cards from graveyards."
fn opponents_not_from_graveyards(q: &dyn Query, asker: &Asker, player: PlayerId, land: ObjectId, _: CardId) -> bool {
    player == asker.controller || !q.obj(land).zone.is(ZoneKind::Graveyard)
}

/// "Lands named Land Play Test Vale can't be played": the face played is named.
fn no_vales(q: &dyn Query, _: &Asker, _: PlayerId, _: ObjectId, face: CardId) -> bool {
    q.card(face).name != VALE.name
}

static WARDEN: CardDef = CardDef {
    name: "Land Play Test Warden",
    types: TypeSet::of(&[CardType::Enchantment]),
    land_play_rules: &[opponents_not_from_graveyards],
    ..CardDef::BLANK
};
static NAMER: CardDef =
    CardDef { name: "Land Play Test Namer", types: TypeSet::of(&[CardType::Enchantment]), land_play_rules: &[no_vales], ..CardDef::BLANK };
/// "You may play lands from your graveyard."
static CRUCIBLE: CardDef = CardDef {
    name: "Land Play Test Crucible",
    types: TypeSet::of(&[CardType::Artifact]),
    play_rules: &[PlayRule::LandsFromGraveyard],
    ..CardDef::BLANK
};
/// "You may play a land from your graveyard", a way a permanent gives.
static PARAGON: CardDef = CardDef {
    name: "Land Play Test Paragon",
    types: TypeSet::of(&[CardType::Enchantment]),
    granted_ways: &[GrantedWay { text: "Play a land from your graveyard.", cost: |_, _, _, _| Some(Cost::NONE), lands: true, permission: true, ..GrantedWay::CAST }],
    ..CardDef::BLANK
};
static WASTE: CardDef = CardDef { name: "Land Play Test Waste", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static VALE: CardDef = CardDef { name: "Land Play Test Vale", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
/// A modal double-faced card: a sorcery whose back face is the Vale.
static MODAL: CardDef = CardDef {
    name: "Land Play Test Modal",
    types: TypeSet::of(&[CardType::Sorcery]),
    has_cost: true,
    back: Some(&VALE),
    back_land: true,
    ..CardDef::BLANK
};
static CASTER: CardDef = CardDef { name: "Land Play Test Caster", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

/// `active`'s precombat main phase, with nothing on the battlefield.
fn game(active: PlayerId) -> (Game, &'static CardTable) {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&WARDEN, &NAMER, &CRUCIBLE, &PARAGON, &WASTE, &VALE, &MODAL, &CASTER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = active;
        t.priority = active;
        t.set_step(Step::PrecombatMain);
    });
    (game, table)
}

/// A card `owner` owns, in `zone`.
fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), owner, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The land plays `player` is offered of `land`, by way number (0 the card itself, `WAY_BACK_FACE` its back face).
fn plays(game: &mut Game, player: PlayerId, land: ObjectId) -> Vec<u8> {
    let mut cx = cx(game);
    priority_options(&mut cx, player).iter().filter(|o| o.kind == ActionKind::PlayLand && o.source == Some(land)).map(|o| o.index).collect()
}

/// The opponent of the Warden's controller may play lands from their graveyard by their Crucible, but not while the Warden is
/// there, a modal double-faced card's land face included; a land from their hand they may still play.
#[test]
fn an_opponent_cannot_play_a_land_from_a_graveyard_by_a_rule() {
    let (mut game, table) = game(THEM);
    put(&mut game, table, &CRUCIBLE, THEM, Zone::BATTLEFIELD);
    let waste = put(&mut game, table, &WASTE, THEM, Zone::graveyard(THEM));
    let modal = put(&mut game, table, &MODAL, THEM, Zone::graveyard(THEM));
    let in_hand = put(&mut game, table, &WASTE, THEM, Zone::hand(THEM));
    assert_eq!(plays(&mut game, THEM, waste), vec![0]);
    assert_eq!(plays(&mut game, THEM, modal), vec![WAY_BACK_FACE]);
    put(&mut game, table, &WARDEN, YOU, Zone::BATTLEFIELD);
    assert!(plays(&mut game, THEM, waste).is_empty());
    assert!(plays(&mut game, THEM, modal).is_empty());
    assert_eq!(plays(&mut game, THEM, in_hand), vec![0]);
}

/// The Warden's controller is no opponent of theirs: they play a land from their graveyard by their Crucible.
#[test]
fn its_controller_still_plays_a_land_from_a_graveyard() {
    let (mut game, table) = game(YOU);
    put(&mut game, table, &CRUCIBLE, YOU, Zone::BATTLEFIELD);
    put(&mut game, table, &WARDEN, YOU, Zone::BATTLEFIELD);
    let waste = put(&mut game, table, &WASTE, YOU, Zone::graveyard(YOU));
    assert_eq!(plays(&mut game, YOU, waste), vec![0]);
}

/// A way a permanent gives to play a land from a graveyard (Serra Paragon's kind) is overruled too (CR 101.2).
#[test]
fn a_way_a_permanent_gives_is_overruled() {
    let (mut game, table) = game(THEM);
    put(&mut game, table, &PARAGON, THEM, Zone::BATTLEFIELD);
    let waste = put(&mut game, table, &WASTE, THEM, Zone::graveyard(THEM));
    assert_eq!(plays(&mut game, THEM, waste), vec![0]);
    put(&mut game, table, &WARDEN, YOU, Zone::BATTLEFIELD);
    assert!(plays(&mut game, THEM, waste).is_empty());
}

/// A permission an effect gave to play a card in a graveyard is overruled; the same permission for a card in exile is not,
/// as the Warden names graveyards alone.
#[test]
fn a_permission_an_effect_gave_is_overruled_in_a_graveyard_alone() {
    let (mut game, table) = game(THEM);
    put(&mut game, table, &WARDEN, YOU, Zone::BATTLEFIELD);
    let in_graveyard = put(&mut game, table, &WASTE, THEM, Zone::graveyard(THEM));
    let in_exile = put(&mut game, table, &WASTE, THEM, Zone::exile(THEM));
    {
        let mut cx = cx(&mut game);
        for card in [in_graveyard, in_exile] {
            cantrip_api::Game::allow_play(&mut cx, card, THEM, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
        }
    }
    assert!(plays(&mut game, THEM, in_graveyard).is_empty());
    assert_eq!(plays(&mut game, THEM, in_exile), vec![0]);
}

/// A Warden that has lost all its abilities forbids nothing (CR 613.1f).
#[test]
fn a_warden_without_its_abilities_forbids_nothing() {
    let (mut game, table) = game(THEM);
    put(&mut game, table, &CRUCIBLE, THEM, Zone::BATTLEFIELD);
    let warden = put(&mut game, table, &WARDEN, YOU, Zone::BATTLEFIELD);
    let waste = put(&mut game, table, &WASTE, THEM, Zone::graveyard(THEM));
    assert!(plays(&mut game, THEM, waste).is_empty());
    {
        let mut cx = cx(&mut game);
        cantrip_api::Game::add_effect(&mut cx, warden, warden, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    }
    assert_eq!(plays(&mut game, THEM, waste), vec![0]);
}

/// A modal double-faced card played as its land face is judged as that face (CR 712.12): a ban on lands of the back face's
/// name keeps it from being played that way, though the card in hand has the front face's name alone (CR 712.8a); a land
/// card of another name is played.
#[test]
fn a_back_face_is_judged_as_the_face_played() {
    let (mut game, table) = game(YOU);
    let modal = put(&mut game, table, &MODAL, YOU, Zone::hand(YOU));
    let waste = put(&mut game, table, &WASTE, YOU, Zone::hand(YOU));
    let vale = put(&mut game, table, &VALE, YOU, Zone::hand(YOU));
    assert_eq!(plays(&mut game, YOU, modal), vec![WAY_BACK_FACE]);
    put(&mut game, table, &NAMER, THEM, Zone::BATTLEFIELD);
    assert!(plays(&mut game, YOU, modal).is_empty());
    assert!(plays(&mut game, YOU, vale).is_empty());
    assert_eq!(plays(&mut game, YOU, waste), vec![0]);
}

/// An effect that has its controller play a land as it resolves (CR 305.1, 608.2g) can't play the opponent's land from their
/// graveyard; one from their hand it can.
#[test]
fn a_land_an_effect_would_play_from_a_graveyard_is_forbidden_too() {
    let (mut game, table) = game(THEM);
    put(&mut game, table, &WARDEN, YOU, Zone::BATTLEFIELD);
    let in_graveyard = put(&mut game, table, &WASTE, THEM, Zone::graveyard(THEM));
    let in_hand = put(&mut game, table, &WASTE, THEM, Zone::hand(THEM));
    let caster = put(&mut game, table, &CASTER, THEM, Zone::STACK);
    let item = stack_item(StackKind::Spell, caster, AbilityRef { card: table.id_of(&CASTER), index: 0 }, THEM, &[]);
    game.state.stack_push(item);
    let serial = game.state.stack.last().expect("the caster").serial;
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
    let frame = game.state.agenda.len() - 1;
    let resolving = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    assert!(!could_play_land_during_resolution(&resolving, in_graveyard));
    assert!(could_play_land_during_resolution(&resolving, in_hand));
}
