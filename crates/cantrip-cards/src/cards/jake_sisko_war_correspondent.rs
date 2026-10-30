use cantrip_api::*;
use cantrip_oracle as oracle;

use cantrip_mechanics::{tokens, triggers};

/// "You gain 1 life. If this is the second time this ability has resolved this turn, investigate." The resolution count is per
/// source ability and turn, not per land event; repeated triggers resolving after the second do not investigate again.
fn landfall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    if g.history().resolutions(item.object, item.ability) == 1 {
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::LANDFALL, landfall, "Landfall — Whenever a land you control enters, you gain 1 life. If this is the second time this ability has resolved this turn, investigate.")],
    ..oracle::JAKE_SISKO_WAR_CORRESPONDENT
};
