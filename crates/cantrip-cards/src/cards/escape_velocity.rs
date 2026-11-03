use cantrip_api::*;
use cantrip_mechanics::{attach, escape};
use cantrip_oracle as oracle;

/// "Enchanted creature gets +1/+0 and has haste." (CR 608.2).
fn enchanted_creature_gets_1_0_enchant(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Enchanted creature gets +1/+0 and has haste." (CR 604.1).
fn enchanted_creature_gets_1_0_apply_1(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Haste);
}

/// "Enchanted creature gets +1/+0 and has haste." (CR 604.1).
fn enchanted_creature_gets_1_0_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let (p, t) = (1i16, 0i16);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_gets_1_0_apply_1,
            "Enchanted creature gets +1/+0 and has haste.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                enchanted_creature_gets_1_0_apply_0,
                "Enchanted creature gets +1/+0 and has haste.",
            )
            .continues(),
    ],
    enchant: Some(target(enchanted_creature_gets_1_0_enchant)),
    ways: &[
        escape::way(
            mana("{1}{R}"),
            2,
            "Escape—{1}{R}, Exile two other cards from your graveyard.",
        ),
    ],
    ..oracle::ESCAPE_VELOCITY
};
