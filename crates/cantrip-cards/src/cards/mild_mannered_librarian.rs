use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{3}{G}: This creature becomes a Werewolf. Put two +1/+1 counters on it and you draw a card. Activate only once."
/// (CR 602.5b). "Becomes a Werewolf" sets its creature type, so it is no longer a Human (CR 205.1a, 613.1d), with no end
/// (CR 611.2a: the duration is indefinite). Each sentence is its own instruction (CR 608.2c); if it has left the battlefield
/// the first two do nothing to it and the controller still draws (CR 400.7).
fn become_a_werewolf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let werewolf = g.words(item.object).subtype(Subtype::Werewolf);
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetCreatureType(werewolf), Duration::Indefinite);
    });
    g.add_counters(item.object, CounterKind::PlusOne, 2);
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{G}"),
        become_a_werewolf,
        "{3}{G}: This creature becomes a Werewolf. Put two +1/+1 counters on it and you draw a card. Activate only once.",
    )
    .once()],
    ..oracle::MILD_MANNERED_LIBRARIAN
};
