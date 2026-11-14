use cantrip_api::*;
use cantrip_mechanics::{attacking_creatures, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

/// "You gain 1 life for each attacking creature" (CR 119.3, 508.1k): every creature attacking as the ability resolves, whoever
/// controls it (the Dragon counts them all, not only its controller's), a creature that has since been removed from combat no longer
/// one. Gained at once, as one life gain.
fn gain_life_for_each_attacking_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let attacking = attacking_creatures(g) as i32;
    g.gain_life(item.controller, attacking);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        gain_life_for_each_attacking_creature,
        "Whenever one or more creatures you control attack, you gain 1 life for each attacking creature.",
    )],
    ..oracle::ANCESTOR_DRAGON
};
