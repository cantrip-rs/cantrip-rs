use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Count each artifact or creature card in the targeted opponent's graveyard once, then add that
/// much colorless mana and gain that much life (CR 608.2c, 106.3, 119.3).
fn spoils(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(opponent) = g.state().target(item, 0) else { return Ok(()) };
    let count = g.state().zone(Zone::graveyard(opponent)).iter().filter(|&&id| {
        let chars = g.chars(id);
        chars.is(CardType::Artifact) || chars.is(CardType::Creature)
    }).count();
    g.add_mana(item.controller, Mana::C, count as u32);
    g.gain_life(item.controller,(count) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], spoils),
    ..oracle::SPOILS_OF_EVIL
};
