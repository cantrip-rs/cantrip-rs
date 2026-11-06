use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::regenerate;

/// "This creature gets +1/+1 as long as you control a Swamp." (CR 604.1).
fn creature_gets_1_1_as_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    source == affected && q.state().battlefield().iter().copied().any(|id| {
        q.controller(id) == q.controller(source)
            && q.chars(id).has_subtype(q.words(source).subtype(Subtype::Swamp))
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{B}"), regenerate::this_effect, "{B}: Regenerate this creature."),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            creature_gets_1_1_as_affects,
            cantrip_mechanics::statics::plus_pt_saturating::<1, 1>,
            "This creature gets +1/+1 as long as you control a Swamp.",
        ),
    ],
    ..oracle::SEDGE_TROLL
};
