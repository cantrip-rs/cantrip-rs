use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, creature_too};
use cantrip_oracle as oracle;

/// "All Forests are 1/1 creatures that are still lands." (CR 604.1).
fn all_forests_1_1_creatures_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Forest))
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Type,
            all_forests_1_1_creatures_affects,
            creature_too,
            "All Forests are 1/1 creatures that are still lands.",
        ),
        static_ability(
                Layer::PtSet,
                all_forests_1_1_creatures_affects,
                base_pt::<1, 1>,
                "All Forests are 1/1 creatures that are still lands.",
            )
            .continues(),
    ],
    ..oracle::LIVING_LANDS
};
