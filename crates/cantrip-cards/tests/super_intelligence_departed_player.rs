//! A recorded upkeep player who has left cannot be instructed to draw (CR 800.4a).
use cantrip_api::*;
use cantrip_engine::{card, cards, Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind, Span, INLINE_TARGETS};

#[test]
fn departed_upkeep_player_is_not_instructed_to_draw() {
    let mut game = RulesGame::new(cards(), &Setup {
        decks: vec![vec![], vec![], vec![]], ..Default::default()
    });
    let departed = PlayerId(0);
    let controller = PlayerId(1);
    let aura = game.state.push_obj(Obj::new(card("Super Intelligence").unwrap(), controller, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, aura);
    let host = game.state.push_obj(Obj::new(card("Ornithopter").unwrap(), controller, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, host);
    game.state.update_obj(aura, |o| o.attached_to = Some(host));
    game.state.see_statics();
    game.state.update_player(departed, |p| { p.lost = true; p.drew_from_empty = false; });
    // The ability triggered during the departed player's upkeep, before control changed and they left.
    let item = StackItem { event_amount: None,
        kind: StackKind::Triggered, object: aura,
        ability: AbilityRef { card: card("Super Intelligence").unwrap(), index: 0 }, controller,
        inline_targets: [Target::None; INLINE_TARGETS], target_count: 0, inline_spec_of: [0; INLINE_TARGETS],
        x: 0, kicked: 0, counted: 0, paid: 0, modes: 0, way: 0, choice: 0,
        subject: Target::Object(host), data: [u32::from(departed.0), 0], members: Span::default(), serial: 0,
        gift: None, shares: StackItem::NO_SHARES, spill: 0, exile_instead: ExileInstead::Never,
        flashback: false, attacking: false, since: 0, tapped_since: 0, copied: false, cast_known: u32::MAX,
        acquired: Acquired::OWN, instance: AbilityAcquisition::OWN, words: Words::NONE,
    };
    game.state.stack_push(item);
    let serial = game.state.stack.last().unwrap().serial;
    game.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    game.run();
    assert!(!game.state.player(departed).drew_from_empty);
}
