use cantrip_api::*;
use cantrip_mechanics::attach;
use cantrip_oracle as oracle;

/// "Enchanted creature gets +1/+1 for each enchantment you control and has first strike." (CR 608.2).
fn enchanted_creature_gets_1_1_enchant(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Enchanted creature gets +1/+1 for each enchantment you control and has first strike." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_1(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::FirstStrike);
}

/// "Enchanted creature gets +1/+1 for each enchantment you control and has first strike." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    let (p, t) = (
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Enchantment)
                    && asker.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) as i16,
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Enchantment)
                    && asker.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) as i16,
    );
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_gets_1_1_apply_1,
            "Enchanted creature gets +1/+1 for each enchantment you control and has first strike.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                enchanted_creature_gets_1_1_apply_0,
                "Enchanted creature gets +1/+1 for each enchantment you control and has first strike.",
            )
            .continues(),
    ],
    enchant: Some(target(enchanted_creature_gets_1_1_enchant)),
    ..oracle::ETHEREAL_ARMOR
};
