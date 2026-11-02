use cantrip_api::*;
use cantrip_mechanics::{attach, escape};
use cantrip_oracle as oracle;

/// "Enchanted creature gets +1/+1 and has vigilance." (CR 608.2).
fn enchanted_creature_gets_1_1_enchant(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Enchanted creature gets +1/+1 and has vigilance." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_1(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Vigilance);
}

/// "Enchanted creature gets +1/+1 and has vigilance." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let (p, t) = (1i16, 1i16);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_gets_1_1_apply_1,
            "Enchanted creature gets +1/+1 and has vigilance.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                enchanted_creature_gets_1_1_apply_0,
                "Enchanted creature gets +1/+1 and has vigilance.",
            )
            .continues(),
    ],
    enchant: Some(target(enchanted_creature_gets_1_1_enchant)),
    ways: &[
        escape::way(
            mana("{W}"),
            2,
            "Escape—{W}, Exile two other cards from your graveyard.",
        ),
    ],
    ..oracle::SENTINELS_EYES
};
