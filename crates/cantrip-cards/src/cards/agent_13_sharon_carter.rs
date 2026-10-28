use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE;
use cantrip_oracle as oracle;

/// "Whenever a creature you control attacks alone, investigate." (CR 506.5, 701.16a): any creature of its controller's, this
/// one included; the Clue is made under the ability's controller.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CREATURE_YOU_CONTROL_ATTACKS_ALONE,
        investigate,
        "Whenever a creature you control attacks alone, investigate.",
    )],
    ..oracle::AGENT_13_SHARON_CARTER
};
