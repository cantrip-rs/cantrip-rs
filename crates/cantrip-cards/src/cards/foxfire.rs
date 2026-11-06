use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Prevent combat damage to and by the affected creature (CR 615.1).
fn combat_damage_to_and_by(_: &dyn Query, _: &Asker, creature: ObjectId, damage: &DamageEvent) -> bool {
    damage.combat && (damage.source == creature || damage.target == Target::Object(creature))
}

/// Untap the target, prevent its combat damage, and schedule the next upkeep draw (CR 608.2c, 611.2a, 603.7b).
fn foxfire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.untap(creature);
        g.end_sentence();
        g.add_effect(item.object, creature, EffectKind::Prevention {
            card: item.ability.card, index: 0, to: Target::None, shield: Shield::All,
        }, Duration::EndOfTurn);
    }
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None },
        item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_CREATURE], foxfire),
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep.")],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents all combat damage dealt to and dealt by that creature",
        damage: "combat damage",
        of_it: Some("to and dealt by it"),
        applies: combat_damage_to_and_by,
        then: None,
        text: "Prevent all combat damage that would be dealt to and dealt by that creature this turn.",
    }],
    ..oracle::FOXFIRE
};
