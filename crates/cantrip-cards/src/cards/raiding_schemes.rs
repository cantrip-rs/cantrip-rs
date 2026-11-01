use cantrip_api::*;
use cantrip_mechanics::{conspire, statics};
use cantrip_oracle as oracle;

/// "Each noncreature spell you cast has conspire" (CR 702.78, 604.1): while this enchantment is on the battlefield each such
/// spell on the stack has the conspire ability granted to it, one instance for each Raiding Schemes (CR 702.78b).
pub static CARD: CardDef = CardDef {
    labels: &["Cast without conspire", "Conspire: tap two untapped creatures you control that share a color with it"],
    delayed: &[conspire::trigger("Conspire (As you cast this spell, you may tap two untapped creatures you control that share a color with it. When you do, copy it and you may choose new targets for the copy.)")],
    statics: &[static_ability(Layer::Ability, noncreature_spell_you_cast, statics::grant_delayed_ability, "Each noncreature spell you cast has conspire.").everywhere()],
    ..oracle::RAIDING_SCHEMES
};

/// A noncreature spell you cast (CR 601.2i, 707.10), judged by its characteristics on the stack.
fn noncreature_spell_you_cast(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    !chars.is(CardType::Creature) && statics::spell_you_cast(view, source, affected)
}
