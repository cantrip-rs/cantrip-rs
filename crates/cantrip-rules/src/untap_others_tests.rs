//! "Untap this artifact during each other player's untap step" (Bender's Waterskin, Victory Chimes, Endbringer, Thousand
//! Moons Infantry) and "untap all permanents you control during each other player's untap step" (Seedborn Muse):
//! `CardDef::untap_in_others_untap_steps` (CR 502.3, 611.3a), over test cards, since no printed card is built yet.

use crate::turn::untap_step;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

/// "Untap this artifact during each other player's untap step."
fn this_artifact(_: &dyn Query, asker: &Asker, object: ObjectId) -> bool {
    object == asker.source
}

/// "Untap all permanents you control during each other player's untap step."
fn permanents_you_control(q: &dyn Query, asker: &Asker, object: ObjectId) -> bool {
    q.state().obj(object).controller == asker.controller
}

static SKIN: CardDef = CardDef {
    name: "Untap Test Waterskin",
    types: TypeSet::of(&[CardType::Artifact]),
    untap_in_others_untap_steps: &[this_artifact],
    ..CardDef::BLANK
};
/// The same, and "this artifact doesn't untap during your untap step" besides (Mana Vault's words).
static FROZEN_SKIN: CardDef = CardDef { name: "Untap Test Frozen Waterskin", keywords: Keywords::of(&[Keyword::DoesNotUntap]), ..SKIN };
/// The same, and "you may choose not to untap this artifact during your untap step" besides.
static OPTIONAL_SKIN: CardDef = CardDef { name: "Untap Test Optional Waterskin", may_not_untap: true, ..SKIN };
static MUSE: CardDef = CardDef {
    name: "Untap Test Muse",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(4),
    untap_in_others_untap_steps: &[permanents_you_control],
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef { name: "Untap Test Land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&SKIN, &FROZEN_SKIN, &OPTIONAL_SKIN, &MUSE, &LAND])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

/// A tapped permanent of `def` under `controller`.
fn tapped(game: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let card = game.cards.id_of(def);
    let id = game.state.push_obj(Obj::new(card, controller, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.update_obj(id, |o| o.tapped = true);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The untap step of `active`'s turn: the untap step's turn-based action alone (CR 502.3).
fn untap_step_of(game: &mut Game, active: PlayerId) {
    game.state.update_turn(|t| {
        t.active = active;
        t.set_step(Step::Untap);
    });
    untap_step(&mut cx(game), active).expect("nothing is asked");
}

fn is_tapped(game: &Game, id: ObjectId) -> bool {
    game.state.obj(id).tapped
}

#[test]
fn this_artifact_untaps_in_each_other_players_untap_step_with_that_players_permanents() {
    let mut game = game();
    let skin = tapped(&mut game, &SKIN, YOU);
    let your_land = tapped(&mut game, &LAND, YOU);
    let their_land = tapped(&mut game, &LAND, THEM);
    untap_step_of(&mut game, THEM);
    // CR 502.3: their permanents untap, and the Waterskin with them; nothing else of yours.
    assert!(!is_tapped(&game, skin));
    assert!(!is_tapped(&game, their_land));
    assert!(is_tapped(&game, your_land));
    // In your own untap step it untaps as any of your permanents does.
    game.state.update_obj(skin, |o| o.tapped = true);
    untap_step_of(&mut game, YOU);
    assert!(!is_tapped(&game, skin));
    assert!(!is_tapped(&game, your_land));
}

#[test]
fn doesnt_untap_during_your_untap_step_does_not_stop_the_untap_in_another_players() {
    let mut game = game();
    let skin = tapped(&mut game, &FROZEN_SKIN, YOU);
    // Ruling (2025-10-02): "doesn't untap during its controller's untap step" is about that step alone.
    untap_step_of(&mut game, YOU);
    assert!(is_tapped(&game, skin));
    untap_step_of(&mut game, THEM);
    assert!(!is_tapped(&game, skin));
}

#[test]
fn an_exerted_permanent_untaps_in_another_players_untap_step_and_stays_exerted() {
    let mut game = game();
    let skin = tapped(&mut game, &SKIN, YOU);
    game.state.update_obj(skin, |o| o.marks.exerted_by = Some(YOU));
    // CR 701.43a: it won't untap during its controller's next untap step, which this isn't.
    untap_step_of(&mut game, THEM);
    assert!(!is_tapped(&game, skin));
    assert_eq!(game.state.obj(skin).marks.exerted_by, Some(YOU));
}

#[test]
fn it_untaps_without_a_choice_though_its_controller_may_keep_it_tapped_in_their_own_untap_step() {
    let mut game = game();
    let skin = tapped(&mut game, &OPTIONAL_SKIN, YOU);
    // Ruling (2025-10-02): "You can't choose to not untap it at that time": nothing is asked (`untap_step_of` panics on a
    // question), and it untaps.
    untap_step_of(&mut game, THEM);
    assert!(!is_tapped(&game, skin));
}

#[test]
fn a_stun_counter_is_removed_instead_of_the_untap_in_another_players_untap_step() {
    let mut game = game();
    let skin = tapped(&mut game, &SKIN, YOU);
    crate::counters::put(&mut cx(&mut game), skin, CounterKind::Stun, 1, THEM, crate::counters::How::Effect);
    // CR 122.1d: the stun counter's replacement applies to the permanent becoming untapped in any step.
    untap_step_of(&mut game, THEM);
    assert!(is_tapped(&game, skin));
    assert_eq!(game.state.obj(skin).counters(CounterKind::Stun), 0);
    untap_step_of(&mut game, THEM);
    assert!(!is_tapped(&game, skin));
}

#[test]
fn permanents_you_control_untap_in_each_other_players_untap_step_whoever_controls_the_source_now() {
    let mut game = game();
    let muse = tapped(&mut game, &MUSE, YOU);
    let your_land = tapped(&mut game, &LAND, YOU);
    let their_land = tapped(&mut game, &LAND, THEM);
    // In your own untap step, your permanents untap as ever, and theirs don't.
    untap_step_of(&mut game, YOU);
    assert!(!is_tapped(&game, muse) && !is_tapped(&game, your_land));
    assert!(is_tapped(&game, their_land));
    game.state.update_obj(muse, |o| o.tapped = true);
    game.state.update_obj(your_land, |o| o.tapped = true);
    untap_step_of(&mut game, THEM);
    assert!(!is_tapped(&game, muse) && !is_tapped(&game, your_land) && !is_tapped(&game, their_land));
    // CR 611.3a: once they control it, it untaps their permanents in your untap step, and yours no longer in theirs.
    game.state.update_obj(muse, |o| o.controller = THEM);
    for id in [muse, your_land, their_land] {
        game.state.update_obj(id, |o| o.tapped = true);
    }
    untap_step_of(&mut game, YOU);
    assert!(!is_tapped(&game, muse) && !is_tapped(&game, their_land) && !is_tapped(&game, your_land));
    game.state.update_obj(your_land, |o| o.tapped = true);
    untap_step_of(&mut game, THEM);
    assert!(is_tapped(&game, your_land));
}

#[test]
fn a_source_that_lost_all_its_abilities_untaps_nothing() {
    let mut game = game();
    let skin = tapped(&mut game, &SKIN, YOU);
    cantrip_api::Game::add_effect(&mut cx(&mut game), skin, skin, EffectKind::LoseAllAbilities, Duration::Indefinite);
    untap_step_of(&mut game, THEM);
    assert!(is_tapped(&game, skin));
}
