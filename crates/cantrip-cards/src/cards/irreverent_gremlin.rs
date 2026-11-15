use cantrip_api::*;
use cantrip_mechanics::effects::may_discard_a_card;
use cantrip_mechanics::history::done_this_turn;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever another creature you control with power 2 or less enters" (CR 603.2, 603.6a), not at all once its controller has
/// done the "do this only once each turn" (CR 603.2h), as Baron Strucker, Hydra Overlord does.
fn another_small_creature_enters_unless_done(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if done_this_turn(q, me) {
        return None;
    }
    triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>(q, me, event)
}

/// "You may discard a card. If you do, draw a card. Do this only once each turn." (CR 701.9, 121.1): the limit is spent by the
/// doing (a discard that happened), not by the triggering, so a "no" leaves the turn unspent, and a triggering that waited
/// while another was done finds it spent.
fn may_discard_to_draw_once_each_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.history().done(item.object, item.ability, item.controller) {
        return Ok(());
    }
    if may_discard_a_card(g, item)? {
        g.note_done(item.object, item.ability, item.controller);
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_small_creature_enters_unless_done),
        may_discard_to_draw_once_each_turn,
        "Whenever another creature you control with power 2 or less enters, you may discard a card. If you do, draw a card. Do this only once each turn.",
    )],
    ..oracle::IRREVERENT_GREMLIN
};
