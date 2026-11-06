use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Combat damage dealt by the creature the effect is about (CR 615.1, 510.2).
fn combat_by(_: &dyn Query, _: &Asker, creature: ObjectId, damage: &DamageEvent) -> bool {
    damage.combat && damage.source == creature
}

/// Prevent all combat damage dealt by the target creature this turn (CR 608.2b, 615.1).
fn prevent_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::Prevention {
            card: item.ability.card, index: 0, to: Target::None, shield: Shield::All,
        }, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{W}{B}").and(TAP), prevent_combat,
        "{W}{B}, {T}: Prevent all combat damage that would be dealt by target creature this turn.")
        .targets(&[targets::CREATURE])],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents combat damage dealt by that creature", damage: "combat damage", of_it: Some("by it"),
        applies: combat_by, then: None,
        text: "Prevent all combat damage that would be dealt by target creature this turn.",
    }],
    ..oracle::LADY_EVANGELA
};
