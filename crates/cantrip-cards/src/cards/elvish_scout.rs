use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target attacking creature you control" (CR 115.1, 506.4).
fn attacking_you_control(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    targets::attacking_creature_filter(q, asker, target)
        && matches!(target, Target::Object(id) if q.controller(id) == asker.controller)
}

/// Prevent combat damage dealt to or by the affected creature (CR 615.1).
fn combat_damage_to_and_by(_: &dyn Query, _: &Asker, creature: ObjectId, damage: &DamageEvent) -> bool {
    damage.combat && (damage.source == creature || damage.target == Target::Object(creature))
}

/// Untap the target, then prevent combat damage to and by it this turn (CR 608.2c, 615.1).
fn untap_and_prevent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.untap(creature);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::Prevention {
        card: item.ability.card, index: 0, to: Target::None, shield: Shield::All,
    }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{G}").and(TAP), untap_and_prevent,
        "{G}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by it this turn.")
        .targets(&[target(attacking_you_control)])],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents all combat damage dealt to and dealt by that creature",
        damage: "combat damage", of_it: Some("to and dealt by it"), applies: combat_damage_to_and_by, then: None,
        text: "Prevent all combat damage that would be dealt to and dealt by it this turn.",
    }],
    ..oracle::ELVISH_SCOUT
};
