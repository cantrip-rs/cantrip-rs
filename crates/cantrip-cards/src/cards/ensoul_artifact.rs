use cantrip_api::*;
use cantrip_mechanics::attach;
use cantrip_oracle as oracle;

/// "Enchanted artifact is a creature with base power and toughness 5/5 in addition to its other types." (CR 608.2).
fn enchanted_artifact_creature_base_power_enchant(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Artifact))
    )
}

/// "Enchanted artifact is a creature with base power and toughness 5/5 in addition to its other types." (CR 604.1).
fn enchanted_artifact_creature_base_power_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.types = chars.types.with(CardType::Creature);
}

/// "Enchanted artifact is a creature with base power and toughness 5/5 in addition to its other types." (CR 604.1).
fn enchanted_artifact_creature_base_power_apply_1(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let (p, t) = (5i16, 5i16);
    chars.power = Some(p);
    chars.toughness = Some(t);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Type,
            attach::attached_object,
            enchanted_artifact_creature_base_power_apply_0,
            "Enchanted artifact is a creature with base power and toughness 5/5 in addition to its other types.",
        ),
        static_ability(
                Layer::PtSet,
                attach::attached_object,
                enchanted_artifact_creature_base_power_apply_1,
                "Enchanted artifact is a creature with base power and toughness 5/5 in addition to its other types.",
            )
            .continues(),
    ],
    enchant: Some(target(enchanted_artifact_creature_base_power_enchant)),
    ..oracle::ENSOUL_ARTIFACT
};
