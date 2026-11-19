use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, sacrifice any number of artifacts, enchantments, and/or tokens, then draw that many cards." (CR 701.21a,
/// 608.2c): the Witchkite may sacrifice itself if it is still an artifact, enchantment or token (it is none, so only what else the
/// player has); "that many" is how many were actually sacrificed.
fn sacrifice_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = effects::sacrifice_any_number_of_artifacts_enchantments_and_tokens(g, item.controller, item.object)?;
    g.draw(item.controller, sacrificed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        sacrifice_then_draw,
        "When this creature enters, sacrifice any number of artifacts, enchantments, and/or tokens, then draw that many cards.",
    )],
    ..oracle::MALEVOLENT_WITCHKITE
};
