use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_ALONE;
use cantrip_mechanics::{effects, pump};
use cantrip_oracle as oracle;

/// "Unbreakable Skin — Whenever Luke Cage attacks alone, he gets +2/+0 and gains indestructible until end of turn." (CR 506.5,
/// 611.2a): both are continuous effects of the resolving ability on Luke Cage, one instruction; nothing if he has left the
/// battlefield since (CR 400.7).
fn unbreakable_skin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        pump(g, item.object, item.object, 2, 0);
        effects::gain_indestructible_until_end_of_turn(g, item.object, item.object);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS_ALONE,
        unbreakable_skin,
        "Unbreakable Skin \u{2014} Whenever Luke Cage attacks alone, he gets +2/+0 and gains indestructible until end of turn.",
    )],
    ..oracle::LUKE_CAGE_POWER_MAN
};
