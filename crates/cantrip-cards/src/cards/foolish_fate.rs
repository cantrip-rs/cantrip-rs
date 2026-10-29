use cantrip_api::*;
use cantrip_mechanics::history::gained_life_this_turn;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Destroy target creature. Infusion \u{2014} If you gained life this turn, that creature's controller loses 3 life." Nothing
/// happens if the creature is no longer a legal target (CR 608.2b). The life is lost by whoever controlled the creature as the
/// spell resolved, whether or not it was destroyed (an indestructible creature, a regeneration shield) and whether or not
/// it died, and only if the spell's controller gained life this turn (CR 608.2c).
fn destroy_and_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.obj(creature).controller;
    g.destroy(creature);
    if gained_life_this_turn(&*g, item.controller) {
        g.lose_life(controller, 3);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], destroy_and_drain), ..oracle::FOOLISH_FATE };
