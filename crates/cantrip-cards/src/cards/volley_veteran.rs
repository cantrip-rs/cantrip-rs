use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{damage_target, permanents_with_subtype_you_control, targets};
use cantrip_oracle as oracle;

/// "It deals damage to target creature an opponent controls equal to the number of Goblins you control" (CR 120.3, 608.2h): the Goblins are
/// counted as the ability resolves, the Veteran itself among them if it is still on the battlefield (ruling 2018-07-13); nothing if the
/// target is no longer legal (CR 608.2b). "Goblins" is read through the Veteran's words (CR 612).
fn blast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let goblin = g.words(item.object).subtype(Subtype::Goblin);
    let goblins = permanents_with_subtype_you_control(g, item.controller, goblin);
    damage_target(g, item, 0, goblins as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, blast, "When this creature enters, it deals damage to target creature an opponent controls equal to the number of Goblins you control.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::VOLLEY_VETERAN
};
