use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target non-Aura permanent" (CR 115.1, 303.4): a permanent, as the layers give it, that is not an Aura. The Aura subtype is
/// no word of this card's text.
fn non_aura_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && !q.chars(id).has_subtype(Subtype::Aura))
}

/// "Each nonland permanent you control becomes a copy of target non-Aura permanent." The target's copiable values as the spell
/// resolves (CR 707.2: its printed values or what it is copying, never counters, tapped state or other effects; the rulings),
/// applied to the nonland permanents its controller has then (CR 611.2c: one that enters later is not affected), the target
/// among them if they control it. The effect states no duration, so each lasts as long as it stays on the battlefield
/// (CR 611.2a-b). They do not enter, so the copied permanent's "enters" abilities do not apply, and an X in its cost is 0
/// (CR 107.3). Each is the same object, so its counters, Auras and other effects still apply (CR 613.1). Nothing if the target is
/// no longer legal (CR 608.2b: the core doesn't resolve it).
fn each_nonland_permanent_you_control_becomes_a_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(original);
    let yours: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.controller(id) == item.controller && !g.chars(id).is(CardType::Land))
        .collect();
    instruction(g, |g| {
        for permanent in yours {
            g.become_copy(item.object, permanent, values, Duration::Indefinite);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(non_aura_permanent)], each_nonland_permanent_you_control_becomes_a_copy),
    ..oracle::MIRRORFORM
};
