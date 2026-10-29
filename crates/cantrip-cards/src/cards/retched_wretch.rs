use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "If it had a -1/-1 counter on it" (CR 603.4, 603.10a): the creature as it last was on the battlefield.
fn had_a_minus_one_counter(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::MinusOne) > 0
}

/// "Return it to the battlefield under its owner's control and it loses all abilities": the card it became in the graveyard,
/// if it is still there (CR 400.7). The effect is on the new object, with no duration, so it lasts as long as the creature
/// stays on the battlefield (CR 611.2a), through the cleanup step (the ruling), and the returned creature has no trigger to
/// return it a second time.
fn return_without_abilities(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = g.found_after_move(item.object) else { return Ok(()) };
    let owner = g.obj(card).owner;
    let wretch = g.put_onto_battlefield(card, owner, false)?;
    // "It" is the permanent returned: a card kept from entering (Grafdigger's Cage, CR 101.2) or sent elsewhere (Containment
    // Priest, CR 614.1c) loses nothing where it is.
    if !g.is_on_battlefield(wretch) {
        return Ok(());
    }
    g.end_sentence();
    instruction(g, |g| g.add_effect(item.object, wretch, EffectKind::LoseAllAbilities, Duration::Indefinite));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        return_without_abilities,
        "When this creature dies, if it had a -1/-1 counter on it, return it to the battlefield under its owner's control and it loses all abilities.",
    )
    .condition(had_a_minus_one_counter)],
    ..oracle::RETCHED_WRETCH
};
