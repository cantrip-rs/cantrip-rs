use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+1 and gains vigilance until end of turn" (CR 611.2a): one instruction, both effects
/// on the one creature if it is still a legal target as the ability resolves (CR 608.2b).
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 1);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Vigilance);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, rally, "{T}: Target creature you control gets +1/+1 and gains vigilance until end of turn. Activate only as a sorcery.")
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed()],
    ..oracle::SURLY_FARRIER
};
