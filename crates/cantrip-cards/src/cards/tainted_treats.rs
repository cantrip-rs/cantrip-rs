use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target artifact or creature. If its mana value was 4 or less, create a Food token" (CR 701.8, 608.2h): the mana value is read
/// as the target was before it is destroyed, and the Food is made whether or not it was destroyed (indestructible, regenerated); a
/// target that is no longer legal does nothing at all (CR 608.2b). The Food is the spell's controller's.
fn destroy_and_make_food_if_cheap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let cheap = g.mana_value(target) <= 4;
    destroy_target(g, item, 0);
    if cheap {
        tokens::create(g, item.controller, &tokens::FOOD, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], destroy_and_make_food_if_cheap),
    ..oracle::TAINTED_TREATS
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
