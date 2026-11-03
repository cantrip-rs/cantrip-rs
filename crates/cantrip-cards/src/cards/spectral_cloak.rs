use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchanted creature has shroud as long as it's untapped." (CR 604.1).
fn enchanted_creature_has_shroud_as_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .and_then(|o: ObjectId| q.state().obj(o).attached_to)
        .is_some_and(|candidate: ObjectId| !(q.state().obj(candidate).tapped)))
        && (q.state().obj(source).attached_to == Some(affected))
}

/// "Enchanted creature has shroud as long as it's untapped." (CR 604.1).
fn enchanted_creature_has_shroud_as_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Shroud);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            enchanted_creature_has_shroud_as_affects,
            enchanted_creature_has_shroud_as_apply_0,
            "Enchanted creature has shroud as long as it's untapped.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::SPECTRAL_CLOAK
};
