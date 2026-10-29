use cantrip_api::*;
use cantrip_mechanics::{hand, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "You may put a creature card with mana value X or less from your hand onto the battlefield tapped and attacking, where X is
/// the number of attacking creatures you control." X is counted as the ability resolves (CR 608.2c), the attackers the
/// Sentry's controller controls then, the Sentry included while it is still attacking; the card's mana value is judged in the
/// hand (`Query::mana_value`, X is 0 there, CR 107.3b). The card enters attacking a player or planeswalker its controller
/// chooses (CR 508.4); it was never declared as an attacker, so nothing that watches attackers being declared sees it.
fn put_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = g
        .state()
        .battlefield()
        .iter()
        .filter(|&&id| g.is_attacking(id) && g.chars(id).is(CardType::Creature) && g.controller(id) == you)
        .count() as u32;
    hand::may_put_onto_battlefield_attacking(g, you, item.object, |g, c| {
        g.chars(c).is(CardType::Creature) && g.mana_value(c) <= x
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        put_attacking,
        "Whenever this creature attacks, you may put a creature card with mana value X or less from your hand onto the battlefield tapped and attacking, where X is the number of attacking creatures you control.",
    )],
    ..oracle::KINSCAER_SENTRY
};
