use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Combat damage dealt by the creature the prevention effect is about (CR 615.1).
fn combat_damage_by(_: &dyn Query, _: &Asker, creature: ObjectId, damage: &DamageEvent) -> bool {
    damage.combat && damage.source == creature
}

/// Prevent its combat damage, then give it toughness equal to its mana value as this resolves (CR 608.2h, 611.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::Prevention {
        card: item.ability.card, index: 0, to: Target::None, shield: Shield::All,
    }, Duration::EndOfTurn);
    g.end_sentence();
    if is_live(g, creature) {
        let value = g.mana_value(creature).min(i32::MAX as u32) as i32;
        g.add_effect(item.object, creature, EffectKind::Pump(0, value), Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_preventions: &[GrantedPreventionDef {
        hint: "Prevent all combat damage that would be dealt by target creature this turn. That creature gets +0/+X until end of turn, where X is its mana value.",
        damage: "damage",
        of_it: None,
        applies: combat_damage_by,
        then: None,
        text: "Prevent all combat damage that would be dealt by target creature this turn. That creature gets +0/+X until end of turn, where X is its mana value.",
    }],
    ..oracle::SUBDUE
};
