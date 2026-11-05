use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Other creatures get -1/-1." (CR 604.1).
fn other_creatures_get_1_1_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(asker.source == affected) && (*so_far).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            other_creatures_get_1_1_affects,
            cantrip_mechanics::statics::plus_pt_saturating::<-1, -1>,
            "Other creatures get -1/-1.",
        ),
    ],
    ..oracle::KAERVEK_THE_SPITEFUL
};
