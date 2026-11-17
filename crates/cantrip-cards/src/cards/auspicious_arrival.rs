use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 until end of turn. Investigate." (CR 611.2a, 701.16a): two sentences, the creature being the spell's
/// only target; if it is no longer a legal target as the spell resolves, the spell does not resolve and no Clue is made (CR 608.2b).
fn pump_and_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<2, 2>(g, item)?;
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], pump_and_investigate),
    ..oracle::AUSPICIOUS_ARRIVAL
};
