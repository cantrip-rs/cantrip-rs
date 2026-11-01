use cantrip_api::*;
use cantrip_mechanics::{casualty, statics};
use cantrip_oracle as oracle;

/// "Each instant and sorcery spell you cast has casualty 1" (CR 702.153, 604.1): while this creature is on the battlefield each
/// such spell on the stack has the casualty ability granted to it, one instance for each Silverquill, so two of them offer
/// the cost twice and each copies for its own payment only (CR 702.153b).
pub static CARD: CardDef = CardDef {
    labels: &["Cast without casualty", "Casualty 1: sacrifice a creature with power 1 or greater"],
    delayed: &[casualty::granted::<1>("Casualty 1 (As you cast this spell, you may sacrifice a creature with power 1 or greater. When you do, copy it and you may choose new targets for the copy.)")],
    statics: &[static_ability(Layer::Ability, statics::instant_or_sorcery_spell_you_cast, statics::grant_delayed_ability, "Each instant and sorcery spell you cast has casualty 1.").everywhere()],
    ..oracle::SILVERQUILL_THE_DISPUTANT
};
