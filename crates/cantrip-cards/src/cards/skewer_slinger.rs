use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This creature deals 1 damage to that creature." (CR 120.3, 509.3b): the creature on the other side of the block the trigger is
/// about; if it has left the battlefield as the ability resolves, it is a new object and `damage` deals nothing to it (CR 400.7).
fn damage_that_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, item.subject, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BLOCKS, triggers::blocks_or_becomes_blocked),
        damage_that_creature,
        "Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.",
    )],
    ..oracle::SKEWER_SLINGER
};
