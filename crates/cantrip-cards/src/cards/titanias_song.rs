use cantrip_api::*;
use cantrip_mechanics::statics::{base_mana_value, creature_too};
use cantrip_oracle as oracle;

fn noncreature_artifact(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Artifact) && !c.is(CardType::Creature)
}
pub static CARD: CardDef = CardDef {
    // One continuous effect applies in layers 4, 6 and 7b; its affected set is
    // recomputed and it lasts through cleanup after Song leaves (CR 611.3a,
    // 613.1d, 613.1f, 613.4b, 613.6, 514.2).
    statics: &[
        static_ability(Layer::Type, noncreature_artifact, creature_too, "").lingers_until_end_of_turn(),
        loses_all_abilities(noncreature_artifact, "").continues(),
        static_ability_for_each(Layer::PtSet, noncreature_artifact, base_mana_value, "").continues(),
    ],
    ..oracle::TITANIAS_SONG
};
