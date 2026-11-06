use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "As long as enchanted land is a basic Mountain, Goblin creatures get +0/+2." (CR 604.1).
fn as_long_as_enchanted_land_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .and_then(|o: ObjectId| q.state().obj(o).attached_to)
        .is_some_and(|candidate: ObjectId| {
            q.chars(candidate).supertypes.basic
                && q
                    .chars(candidate)
                    .has_subtype(q.words(asker.source).subtype(Subtype::Mountain))
        }))
        && (((*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Goblin))
            && (*so_far).is(CardType::Creature)))
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            as_long_as_enchanted_land_affects,
            cantrip_mechanics::statics::plus_pt_saturating::<0, 2>,
            "As long as enchanted land is a basic Mountain, Goblin creatures get +0/+2.",
        ),
    ],
    enchant: Some(targets::LAND),
    ..oracle::GOBLIN_CAVES
};
