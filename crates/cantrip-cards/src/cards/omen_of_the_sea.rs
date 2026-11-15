use cantrip_api::*;
use cantrip_mechanics::{effects, scry, triggers};
use cantrip_oracle as oracle;

/// "{2}{U}, Sacrifice this enchantment: Scry 2." (CR 608.2).
fn scry_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    scry(g, item.controller, item.object, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::scry_then_draw::<2>,
            "When this enchantment enters, scry 2, then draw a card.",
        ),
        ability(
            cost("{2}{U}").and(SACRIFICE_THIS),
            scry_2,
            "{2}{U}, Sacrifice this enchantment: Scry 2.",
        ),
    ],
    ..oracle::OMEN_OF_THE_SEA
};
