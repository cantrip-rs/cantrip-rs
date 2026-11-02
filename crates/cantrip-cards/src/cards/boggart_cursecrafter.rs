use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever another Goblin you control dies" (CR 700.4, 603.10a): a permanent other than this one, a creature or not (CR 109.2,
/// 205.3), that was a Goblin, under this ability's controller's control, as it last was on the battlefield. "Goblin" is a word
/// of this card's text (CR 612).
fn another_goblin_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, .. } if *old == me.source => None,
        _ => triggers::of_subtype_you_control_dies::<{ lands::sub(Subtype::Goblin) }>(q, me, event),
    }
}

/// "This creature deals 1 damage to each opponent."
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_goblin_you_control_dies),
        ping,
        "Whenever another Goblin you control dies, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::BOGGART_CURSECRAFTER
};
