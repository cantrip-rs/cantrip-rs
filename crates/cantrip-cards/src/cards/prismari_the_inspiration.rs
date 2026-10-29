use cantrip_api::*;
use cantrip_mechanics::{statics, storm, triggers};
use cantrip_oracle as oracle;

/// "Instant and sorcery spells you cast have storm" (CR 702.40, 604.1): while this creature is on the battlefield each such spell
/// on the stack has the storm trigger granted to it, one instance for each Prismari, so two of them trigger twice (the ruling: each
/// instance of storm triggers separately).
pub static CARD: CardDef = CardDef {
    abilities: &[triggers::ward_life::<5>("Ward\u{2014}Pay 5 life.")],
    delayed: &[storm::trigger("Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)")],
    statics: &[static_ability(Layer::Ability, statics::instant_or_sorcery_spell_you_cast, statics::grant_delayed_ability, "Instant and sorcery spells you cast have storm.").everywhere()],
    ..oracle::PRISMARI_THE_INSPIRATION
};
