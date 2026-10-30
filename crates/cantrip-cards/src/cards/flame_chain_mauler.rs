use cantrip_api::*;
use cantrip_mechanics::{effects, pump};
use cantrip_oracle as oracle;

/// "This creature gets +1/+0 and gains menace until end of turn" (CR 611.2a, 613.4c, 702.111): both continuous effects come
/// from the ability's source and do nothing once it has left the battlefield (CR 400.7).
fn pump_and_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        pump(g, item.object, item.object, 1, 0);
        if g.is_on_battlefield(item.object) {
            effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Menace);
        }
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{R}"), pump_and_menace, "{1}{R}: This creature gets +1/+0 and gains menace until end of turn.")],
    ..oracle::FLAME_CHAIN_MAULER
};
