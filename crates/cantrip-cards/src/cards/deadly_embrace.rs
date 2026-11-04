use cantrip_api::*;
use cantrip_mechanics::{destroy_target, history, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature an opponent controls. Then draw a card for each creature that died this turn." (CR 701.8, 700.4,
/// 608.2c): the count is read after the destruction, so the target counts if it died, and every creature that died earlier this
/// turn, whoever controlled it (a token that has ceased to exist too); a creature that survived (indestructible) is not counted.
/// With the target illegal the spell does not resolve at all and nothing is drawn (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    let died = history::creatures_died_this_turn(g, None);
    g.draw(item.controller, died as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_AN_OPPONENT_CONTROLS], effect), ..oracle::DEADLY_EMBRACE };
