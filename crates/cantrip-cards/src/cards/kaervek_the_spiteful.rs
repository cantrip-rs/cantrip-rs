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

/// "Other creatures get -1/-1." (CR 604.1).
fn other_creatures_get_1_1_apply_0(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let (p, t) = (-1, -1);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            other_creatures_get_1_1_affects,
            other_creatures_get_1_1_apply_0,
            "Other creatures get -1/-1.",
        ),
    ],
    ..oracle::KAERVEK_THE_SPITEFUL
};
