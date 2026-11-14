use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever another creature you control dies" (CR 700.4, 603.10a), carrying whether it was attacking as it last was on the
/// battlefield (CR 506.3, 506.4): combat forgets it the moment it dies, and the ability resolves after.
fn another_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::another_creature_you_control_dies(q, me, event)?;
    let Event::ZoneChange { old, .. } = event else { return None };
    Some(Fired { subject: Target::None, data: [q.was_attacking(*old) as u32, 0] })
}

/// "Draw a card if it was attacking. Otherwise, Garna deals 1 damage to each opponent."
fn draw_if_it_was_attacking_otherwise_ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.data[0] == 1 {
        effects::draw(g, item, 1)
    } else {
        effects::damage_each_opponent(g, item, 1);
        Ok(())
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_creature_you_control_dies),
        draw_if_it_was_attacking_otherwise_ping,
        "Whenever another creature you control dies, draw a card if it was attacking. Otherwise, Garna deals 1 damage to each opponent.",
    )],
    ..oracle::GARNA_BLOODFIST_OF_KELD
};
