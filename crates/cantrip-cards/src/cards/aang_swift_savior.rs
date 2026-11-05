use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, triggers::THIS_ENTERS, targets};
use cantrip_oracle as oracle;

/// "Up to one other target creature or spell": a creature on the battlefield, or a spell on the stack, other than Aang.
fn other_creature_or_spell(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    match t {
        Target::Object(id) if id != asker.source => {
            (targets::permanent(q, id) && q.chars(id).is(CardType::Creature))
                || q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id)
        }
        _ => false,
    }
}

/// "Put a +1/+1 counter on each tapped creature you control."
fn counters_on_tapped_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tapped: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.obj(id).tapped && g.chars(id).is(CardType::Creature))
        .collect();
    // All at once (CR 608.2, 101.4).
    let puts: Vec<(ObjectId, CounterKind, i64)> = tapped.into_iter().map(|id| (id, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::THIS_ATTACKS, counters_on_tapped_creatures, "Whenever Aang and La attack, put a +1/+1 counter on each tapped creature you control.")],
    ..oracle::AANG_AND_LA_OCEANS_FURY
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, airbend_targets, "When Aang enters, airbend up to one other target creature or spell.")
            .targets(&[up_to_one(other_creature_or_spell)]),
        ability(cost("{8}"), cantrip_mechanics::effects::transform_self, "Waterbend {8}: Transform Aang. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)")
            .waterbend(),
    ],
    back: Some(&BACK),
    ..oracle::AANG_SWIFT_SAVIOR
};
