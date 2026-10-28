use cantrip_api::*;
use cantrip_mechanics::{damage_divided, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When Gandalf enters, he deals 3 damage divided as you choose among one, two, or three targets" (CR 601.2d): the targets
/// and each one's share are fixed as the ability is put on the stack, and a target that has since become illegal is left out
/// with its share (CR 608.2b).
fn divided_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_divided(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, divided_damage, "When Gandalf enters, he deals 3 damage divided as you choose among one, two, or three targets.")
        .targets(&[targets::ANY.between(1, 3)])
        .divide(Divide { total: 3, of_x: None })],
    ..oracle::GANDALF_SPARK_STARTER
};
