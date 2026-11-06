use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::statics::plus_pt_saturating;

/// "This creature gets +1/+2 as long as you control a Forest." (CR 604.1).
fn creature_gets_1_2_as_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.controller(id) == Some(asker.controller).unwrap_or(PlayerId(u8::MAX))
                && q
                    .chars(id)
                    .has_subtype(q.words(asker.source).subtype(Subtype::Forest))
        })
        .count() >= 1) && (source == affected)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            creature_gets_1_2_as_affects,
            plus_pt_saturating::<1, 2>,
            "This creature gets +1/+2 as long as you control a Forest.",
        ),
    ],
    ..oracle::KIRD_APE
};
