use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Artifact creatures you control gain menace until end of turn" (CR 611.2c, 702.111): determine the affected creatures
/// as this ability resolves; ones that arrive later are not affected.
fn artifact_creatures_gain_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<_> = g.permanents_of(item.controller).into_iter().filter(|&id| {
        let chars = g.chars(id);
        chars.is(CardType::Artifact) && chars.is(CardType::Creature)
    }).collect();
    instruction(g, |g| {
        for creature in creatures {
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Menace);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}"), artifact_creatures_gain_menace,
        "{2}: Artifact creatures you control gain menace until end of turn. (A creature with menace can't be blocked except by two or more creatures.)")],
    ..oracle::ORGANIC_AVULSION_UNIT
};
