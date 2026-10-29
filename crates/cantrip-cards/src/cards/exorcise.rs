use cantrip_api::*;
use cantrip_mechanics::exile_target;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_ENCHANTMENT_OR_CREATURE_WITH_POWER_4_OR_GREATER], effect),
    ..oracle::EXORCISE
};
