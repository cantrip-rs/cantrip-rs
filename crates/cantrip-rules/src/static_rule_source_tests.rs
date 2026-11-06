//! Static rules function on emblems in the command zone (CR 114.4), and on
//! permanents with their abilities. Gideon of the Trials and Platinum Angel
//! compose the same loss/win rules with different sources and conditions.
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const OPPONENT: PlayerId = PlayerId(1);

fn protects(_: &dyn Query, asker: &Asker, player: PlayerId, _: LossReason) -> bool {
    player == asker.controller
}
fn forbids(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    q.state().opponents(asker.controller).any(|p| p == player)
}
static EMBLEM: CardDef = CardDef { name: "Static Rule Emblem", loss_rules: &[protects], win_rules: &[forbids], ..CardDef::BLANK };
static ANGEL: CardDef = CardDef {
    name: "Static Rule Angel", types: TypeSet::of(&[CardType::Creature]), power: Some(4), toughness: Some(4),
    loss_rules: &[protects], win_rules: &[forbids], ..CardDef::BLANK
};

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&EMBLEM, &ANGEL])));
    Game::new(cards, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}
fn put(g: &mut Game, def: &'static CardDef, player: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, zone));
    g.state.zone_push(zone, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

#[test]
fn an_emblem_protects_its_controller_and_stops_an_opponent_winning() {
    let mut g = game();
    put(&mut g, &EMBLEM, YOU, Zone::COMMAND);
    for reason in [LossReason::Life, LossReason::EmptyLibrary, LossReason::Poison, LossReason::Effect] {
        assert!(g.doesnt_lose(YOU, reason));
        assert!(!g.doesnt_lose(OPPONENT, reason));
    }
    assert!(g.cant_win(OPPONENT));
    assert!(!g.cant_win(YOU));
    crate::ops::lose_game(&mut cx(&mut g), YOU);
    crate::ops::win_game(&mut cx(&mut g), OPPONENT);
    assert!(!g.state.player(YOU).lost);
    assert!(!g.state.is_over());
    crate::ops::concede(&mut cx(&mut g), YOU);
    assert!(g.state.player(YOU).lost, "concession cannot be prevented (CR 104.3a)");
}

#[test]
fn a_commander_in_the_command_zone_has_no_battlefield_rules() {
    let mut g = game();
    put(&mut g, &ANGEL, YOU, Zone::COMMAND);
    assert!(!g.doesnt_lose(YOU, LossReason::Effect));
    assert!(!g.cant_win(OPPONENT));
}

#[test]
fn battlefield_rules_still_end_when_the_permanent_loses_abilities() {
    let mut g = game();
    let angel = put(&mut g, &ANGEL, YOU, Zone::BATTLEFIELD);
    assert!(g.doesnt_lose(YOU, LossReason::Effect));
    assert!(g.cant_win(OPPONENT));
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect {
        source: angel, controller: YOU, timestamp, affected: angel,
        kind: EffectKind::LoseAllAbilities, duration: Duration::Indefinite, words: Words::NONE,
    });
    assert!(!g.doesnt_lose(YOU, LossReason::Effect));
    assert!(!g.cant_win(OPPONENT));
}
