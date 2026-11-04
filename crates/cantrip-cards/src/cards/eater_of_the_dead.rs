use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Check the source's current or last-known tapped state on resolution; exile and untap are one instruction (CR 608.2h, 608.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.obj(item.object).tapped {
        let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
        instruction(g, |g| {
            g.exile_all(&[card]);
            if g.is_on_battlefield(item.object) {
                g.untap(item.object);
            }
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        Cost::mana(ManaCost::ZERO),
        effect,
        "{0}: If this creature is tapped, exile target creature card from a graveyard and untap this creature.",
    ).targets(&[targets::CREATURE_CARD_IN_A_GRAVEYARD])],
    ..oracle::EATER_OF_THE_DEAD
};
