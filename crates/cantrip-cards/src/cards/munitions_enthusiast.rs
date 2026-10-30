use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers::THIS_DIES};
use cantrip_oracle as oracle;

/// "When this creature dies, it deals 1 damage to any target." The target is chosen as the trigger is put on the stack;
/// the source need not remain on the battlefield (CR 603.3d, 113.7a).
fn deals_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, deals_one, "When this creature dies, it deals 1 damage to any target.")
        .targets(&[targets::ANY])],
    ..oracle::MUNITIONS_ENTHUSIAST
};
