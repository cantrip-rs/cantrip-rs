use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "You may have this creature enter as a copy of any creature on the battlefield." As Clone's (CR 614.1c, 707.2): a choice,
/// not a target, taking only the copiable values, with no exception; declining leaves the printed 0/0 that state-based
/// actions put into the graveyard (CR 704.5f).
fn enter_as_copy(g: &mut dyn Game, entry: &mut Entering) -> Result<(), Suspend> {
    cantrip_mechanics::copy::enter_as_copy(g, entry, |c| c.is(CardType::Creature), |_, copy| copy)
}

/// "Put a +1/+1 counter on target nonlegendary creature you control. Each other creature you control becomes a copy of that
/// creature until end of turn." The counter goes on first, but it is no copiable value (CR 707.2), so the copies are of the
/// printed (or copied) creature; the creatures that become copies are those you control as the ability resolves (CR 611.2c),
/// and each keeps its own counters and effects (CR 613.1). Nothing if the target is no longer legal (CR 608.2b: the core
/// doesn't resolve it).
fn renew(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(original, CounterKind::PlusOne, 1);
    let values = g.copiable_values(original);
    let others: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != original && g.controller(id) == item.controller && g.chars(id).is(CardType::Creature))
        .collect();
    cantrip_mechanics::copy::become_copies(g, item.object, &others, values, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(enter_as_copy),
    // Renew is an ability word (CR 207.2c): the ability is an ordinary activated ability of the card in the graveyard.
    abilities: &[ability(cost("{2}{U}").and(EXILE_THIS), renew, "Renew \u{2014} {2}{U}, Exile this card from your graveyard: Put a +1/+1 counter on target nonlegendary creature you control. Each other creature you control becomes a copy of that creature until end of turn. Activate only as a sorcery.")
        .from(ZoneKind::Graveyard)
        .sorcery_speed()
        .targets(&[targets::NONLEGENDARY_CREATURE_YOU_CONTROL])],
    ..oracle::NAGA_FLESHCRAFTER
};
