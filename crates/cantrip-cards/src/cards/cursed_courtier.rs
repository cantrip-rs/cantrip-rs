use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, create a Cursed Role token attached to it." The Role is created and attached to the Courtier by
/// the trigger's effect, so nothing is targeted; its Enchanted creature is 1/1 sets the base power and toughness (CR 613.4b,
/// 303.4). With the Courtier already gone on resolution there is nothing to attach it to and no Role is created (CR 303.4g).
fn cursed_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_role(g, &tokens::CURSED_ROLE, item.controller, item.object)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        cursed_role,
        "When this creature enters, create a Cursed Role token attached to it.",
    )],
    ..oracle::CURSED_COURTIER
};

pub static EXTRA: &[&CardDef] = &[&tokens::CURSED_ROLE];
