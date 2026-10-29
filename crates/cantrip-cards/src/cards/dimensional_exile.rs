use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Enchant basic land you control" (CR 303.4, 205.4a): a land under the Aura's controller's control with the basic supertype,
/// judged as the Aura is cast and again as it resolves (CR 608.2b).
fn basic_land_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::LAND_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).supertypes.basic)
}

/// "Exile target creature an opponent controls until this Aura leaves the battlefield" (CR 610.3). With the Aura already gone
/// as the ability resolves, nothing is exiled (CR 610.3c).
fn exile_until_it_leaves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target_until_leaves(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(target(basic_land_you_control)),
    abilities: &[triggered(
        THIS_ENTERS,
        exile_until_it_leaves,
        "When this Aura enters, exile target creature an opponent controls until this Aura leaves the battlefield.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::DIMENSIONAL_EXILE
};
