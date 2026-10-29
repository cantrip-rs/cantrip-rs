use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{effects, sneak, targets};
use cantrip_oracle as oracle;

/// "Another target creature you control gains deathtouch until end of turn" (CR 611.2a, 702.2), if it is still a legal target
/// as this resolves (CR 608.2b). One trigger for two events (CR 603.2): he enters, sneaked in or not, or attacks.
fn gain_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Deathtouch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{3}{B}"), "Sneak {3}{B}")],
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        gain_deathtouch,
        "Whenever Shredder enters or attacks, another target creature you control gains deathtouch until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::SHREDDER_UNRELENTING
};
