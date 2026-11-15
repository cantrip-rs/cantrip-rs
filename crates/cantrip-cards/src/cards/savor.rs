use cantrip_api::*;
use cantrip_mechanics::{pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature gets -2/-2 until end of turn. Create a Food token." With its one target illegal the whole spell is
/// countered (CR 608.2b), so no Food is made; otherwise both instructions are followed in order (CR 608.2c). The Food is
/// made whether or not the creature dies.
fn savor(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        pump(g, item.object, creature, -2, -2);
    }
    tokens::create_a_food_token(g, item)
}

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], savor), ..oracle::SAVOR };
