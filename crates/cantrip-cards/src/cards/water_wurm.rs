use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature gets +0/+1 as long as an opponent controls an Island." (CR 604.1).
fn creature_gets_0_1_as_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .into_iter()
        .any(|bound_0: PlayerId| {
            q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.controller(id) == Some(bound_0).unwrap_or(PlayerId(u8::MAX))
                        && q
                            .chars(id)
                            .has_subtype(q.words(asker.source).subtype(Subtype::Island))
                })
                .count() >= 1
        })) && (source == affected)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            creature_gets_0_1_as_affects,
            cantrip_mechanics::statics::plus_pt_saturating::<0, 1>,
            "This creature gets +0/+1 as long as an opponent controls an Island.",
        ),
    ],
    ..oracle::WATER_WURM
};
