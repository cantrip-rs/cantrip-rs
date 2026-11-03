use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

/// "When this Aura enters, target creature can't block this turn. Enchanted creature gets +1/+1 and has haste. (It can attack and {T} no matter when it came under your control.)" (CR 608.2).
fn target_creature_cant_block_enchanted_enchant(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && asker.controller == q.controller(id)))
    )
}

/// "When this Aura enters, target creature can't block this turn." (CR 115.1).
fn target_creature_cant_block_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "When this Aura enters, target creature can't block this turn." (CR 608.2, 611.2a).
fn target_creature_cant_block(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(
        g,
        |g| {
            g.add_effect(
                item.object,
                target,
                EffectKind::CantBlock,
                Duration::EndOfTurn,
            );
        },
    );
    Ok(())
}

/// "Enchanted creature gets +1/+1 and has haste." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_1(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Haste);
}

/// "Enchanted creature gets +1/+1 and has haste." (CR 604.1).
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
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                target_creature_cant_block,
                "When this Aura enters, target creature can't block this turn.",
            )
            .targets(&[target(target_creature_cant_block_target_0)]),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_gets_1_1_apply_1,
            "Enchanted creature gets +1/+1 and has haste.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                enchanted_creature_gets_1_1_apply_0,
                "Enchanted creature gets +1/+1 and has haste.",
            )
            .continues(),
    ],
    enchant: Some(target(target_creature_cant_block_enchanted_enchant)),
    ..oracle::CARTOUCHE_OF_ZEAL
};
