use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Each creature you control assigns combat damage equal to its toughness rather than its power" (CR 510.1a).
fn your_creature(q: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    q.controller(creature) == asker.controller && q.chars(creature).is(CardType::Creature)
}

/// "Target creature with defender" (CR 115.1).
fn creature_with_defender(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).has(Keyword::Defender))
}

/// "{G}: Target creature with defender can attack this turn as though it didn't have defender" (CR 702.3b, 609.4).
fn attack_despite_defender(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::CanAttackDespiteDefender, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{G}"), attack_despite_defender, "{G}: Target creature with defender can attack this turn as though it didn't have defender.")
            .targets(&[target(creature_with_defender)]),
        ability(cost("{2}{G}"), effects::creatures_you_control_get::<0, 1>, "{2}{G}: Creatures you control get +0/+1 until end of turn."),
    ],
    damage_by_toughness: &[your_creature],
    ..oracle::ASSAULT_FORMATION
};
