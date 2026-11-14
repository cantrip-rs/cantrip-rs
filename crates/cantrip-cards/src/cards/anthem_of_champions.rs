use cantrip_api::*;
use cantrip_mechanics::statics::CREATURES_YOU_CONTROL_GET_PLUS_ONE;
use cantrip_oracle as oracle;

/// "Creatures you control get +1/+1." A static ability (CR 604.1) whose effect modifies power and toughness in layer 7c (CR
/// 613.4c), after Humility's or Opalescence's base 1/1 and mana value in 7b. An enchantment, so it pumps creatures only, and
/// the controller's: a creature an opponent controls gets nothing, and one that changes controllers changes with it.
pub static CARD: CardDef = CardDef { statics: &[CREATURES_YOU_CONTROL_GET_PLUS_ONE], ..oracle::ANTHEM_OF_CHAMPIONS };
