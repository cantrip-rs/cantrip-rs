use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target non-Aura permanent you control" (CR 115.1, 608.2b): checked again as the spell resolves.
fn non_aura_permanent_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_you_control_filter(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).has_subtype(Subtype::Aura))
}

/// "Create a token that's a copy of target non-Aura permanent you control, except it's a 0/0 Fractal creature in addition to its
/// other types. Put six +1/+1 counters on it." (CR 707.2, 707.9b, 111.1, 122.1): the exception is part of the copy's copiable
/// values, so a copy of the token is a 0/0 Fractal creature too. The counters are put on every token made, together. Nothing
/// if the target is no longer legal (CR 608.2b).
fn copy_as_fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(permanent).with_type(CardType::Creature).with_subtype(Subtype::Fractal).sized(0, 0);
    let made = g.create_token_copies_of(values, item.controller, 1)?;
    g.end_sentence();
    let puts: Vec<(ObjectId, CounterKind, i16)> = made.iter().map(|&token| (token, CounterKind::PlusOne, 6)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(non_aura_permanent_you_control)], copy_as_fractal),
    ..oracle::APPLIED_GEOMETRY
};
