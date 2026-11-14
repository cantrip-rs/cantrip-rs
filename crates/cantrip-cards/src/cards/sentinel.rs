use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// A creature blocking or blocked by this creature (CR 115.1, 509.1g–h).
fn opposing_combatant(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    matches!(target, Target::Object(id) if targets::permanent(q, id) && q.chars(id).is(CardType::Creature)
        && (q.creatures_blocking(asker.source).contains(&id) || q.creatures_blocked_by(asker.source).contains(&id)))
}

/// Set this creature's base toughness to one plus that creature's power, indefinitely (CR 608.2h, 613.4b).
fn set_toughness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let power = g.chars(creature).power.unwrap_or_default();
    g.add_effect(item.object, item.object, EffectKind::SetToughness(power + 1), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{0}"), set_toughness,
        "{0}: This creature's base toughness becomes equal to 1 plus the power of target creature blocking or blocked by this creature.")
        .targets(&[target(opposing_combatant)])],
    ..oracle::SENTINEL
};
