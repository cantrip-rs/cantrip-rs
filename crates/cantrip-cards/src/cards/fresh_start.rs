use cantrip_api::*;
use cantrip_mechanics::attach;
use cantrip_oracle as oracle;

/// "Enchanted creature gets -5/-0 and loses all abilities." (CR 608.2).
fn enchanted_creature_gets_5_0_enchant(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Enchanted creature gets -5/-0 and loses all abilities." (CR 604.1).
fn enchanted_creature_gets_5_0_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let (p, t) = (-5, 0i16);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

pub static CARD: CardDef = CardDef {
    statics: &[
        loses_all_abilities(
            attach::attached_object,
            "Enchanted creature gets -5/-0 and loses all abilities.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                enchanted_creature_gets_5_0_apply_0,
                "Enchanted creature gets -5/-0 and loses all abilities.",
            )
            .continues(),
    ],
    enchant: Some(target(enchanted_creature_gets_5_0_enchant)),
    ..oracle::FRESH_START
};
