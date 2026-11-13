use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, creature_too};
use cantrip_oracle as oracle;

/// "All lands are 1/1 creatures that are still lands." (CR 604.1).
fn all_lands_1_1_creatures_affects(
    _: &dyn LayerQuery,
    _: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    (*so_far).is(CardType::Land)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Type,
            all_lands_1_1_creatures_affects,
            creature_too,
            "All lands are 1/1 creatures that are still lands.",
        ),
        static_ability(
                Layer::PtSet,
                all_lands_1_1_creatures_affects,
                base_pt::<1, 1>,
                "All lands are 1/1 creatures that are still lands.",
            )
            .continues(),
    ],
    ..oracle::LIVING_PLANE
};
