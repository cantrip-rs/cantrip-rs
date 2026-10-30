use cantrip_api::*;
use cantrip_mechanics::effects::sacrifice_to_destroy_artifact_or_enchantment;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[sacrifice_to_destroy_artifact_or_enchantment(1, "{1}, Sacrifice this creature: Destroy target artifact or enchantment.")],
    ..oracle::CATHAR_COMMANDO
};
