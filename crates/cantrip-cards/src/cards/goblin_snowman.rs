use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// A creature blocks when declared as a blocker (CR 509.1i).
fn blocks(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Blocks { blocker, declared: true, .. } if *blocker == asker.source => Fired::about(Target::Object(*blocker)),
        _ => None,
    }
}

/// Prevent combat damage to or from the creature (CR 615.1).
fn combat_to_and_by(_: &dyn Query, _: &Asker, creature: ObjectId, damage: &DamageEvent) -> bool {
    damage.combat && (damage.source == creature || damage.target == Target::Object(creature))
}

fn prevent_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = item.subject.object() {
        g.add_effect(item.object, creature, EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All }, Duration::EndOfTurn);
    }
    Ok(())
}

/// A creature this source is currently blocking (CR 506.4).
fn creature_blocked_by_this(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.chars(id).is(CardType::Creature)
        && q.state().blocks.iter().any(|b| b.attacker == id && b.blocker == asker.source))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::BLOCKS, blocks), prevent_combat, "Whenever this creature blocks, prevent all combat damage that would be dealt to and dealt by it this turn."),
        ability(TAP, effects::damage_first_target::<1>, "{T}: This creature deals 1 damage to target creature it's blocking.")
            .targets(&[target(creature_blocked_by_this)]),
    ],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents all combat damage dealt to and dealt by this creature",
        damage: "combat damage",
        of_it: Some("to and dealt by it"),
        applies: combat_to_and_by,
        then: None,
        text: "Whenever this creature blocks, prevent all combat damage that would be dealt to and dealt by it this turn.",
    }],
    ..oracle::GOBLIN_SNOWMAN
};
