use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

fn board() -> (Game, ObjectId) {
    static SOURCE: CardDef = CardDef { name: "Wide numeric source", ..CardDef::BLANK };
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    let source = g.state.push_obj(Obj::new(cards.id_of(&SOURCE), PlayerId(0), Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    g.state.frame_push(Frame::new(FrameKind::Act { player: PlayerId(0) }));
    (g, source)
}
fn cx(g: &mut Game) -> Cx<'_> { Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None } }
fn ask(g: &mut Game, source: ObjectId, least: u32, most: u32) -> Result<u32, Suspend> {
    cantrip_mechanics::choose::number(&mut cx(g), PlayerId(0), source, least, most, "Put counters")
}
#[test]
fn wide_number_offers_the_full_legal_engine_x_range() {
    let (mut g, source) = board();
    assert!(ask(&mut g, source, 0, u32::from(u16::MAX)).is_err());
    assert!(matches!(g.state.agenda[0].pending.as_ref().unwrap().kind, RequestKind::ChooseNumber { source: id, least: 0, most: 65535, heading: "Put counters" } if id == source));
}
#[test]
fn wide_number_preserves_u32_endpoints_and_exact_replayed_answers() {
    let (mut g, source) = board();
    assert!(ask(&mut g, source, 1, u32::MAX).is_err());
    let request = g.state.agenda[0].pending.clone().unwrap();
    assert!(crate::check_answer(&g, &request, &Answer::Announce(0)).is_err());
    for value in [1, 300, 65535, u32::MAX] {
        crate::check_answer(&g, &request, &Answer::Announce(value)).unwrap();
        g.state.update_frame(0, |f| f.answers = vec![(Some(PlayerId(0)), Answer::Announce(value))]);
        assert_eq!(ask(&mut g, source, 1, u32::MAX), Ok(value));
    }
}
#[test]
fn wide_number_small_choice_control_keeps_its_bounds_and_metadata() {
    let (mut g, source) = board(); assert!(ask(&mut g, source, 4, 7).is_err());
    let request = g.state.agenda[0].pending.clone().unwrap();
    assert!(matches!(request.kind, RequestKind::ChooseNumber { source: id, least: 4, most: 7, heading: "Put counters" } if id == source));
    assert!(crate::check_answer(&g, &request, &Answer::Announce(3)).is_err());
    assert!(crate::check_answer(&g, &request, &Answer::Announce(8)).is_err());
}
