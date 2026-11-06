use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature has banding as long as you control a Plains." (CR 604.1).
fn creature_has_banding_as_long_affects(
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
                    .has_subtype(q.words(asker.source).subtype(Subtype::Plains))
        })
        .count() >= 1) && (source == affected)
}

/// "This creature has banding as long as you control a Plains." (CR 604.1).
fn creature_has_banding_as_long_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Banding);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            creature_has_banding_as_long_affects,
            creature_has_banding_as_long_apply_0,
            "This creature has banding as long as you control a Plains.",
        ),
    ],
    ..oracle::DIRE_WOLVES
};
