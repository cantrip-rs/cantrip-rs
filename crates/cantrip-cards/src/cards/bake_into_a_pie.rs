use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature. Create a Food token." (CR 701.8, 111.10b): the Food is made by the spell's controller whether or not
/// the creature was destroyed (an indestructible one survives), but not at all if the target was illegal on resolution and the
/// spell was countered by the rules (CR 608.2b).
fn destroy_and_make_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_first_target(g, item)?;
    tokens::create_a_food_token(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], destroy_and_make_food), ..oracle::BAKE_INTO_A_PIE };

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
