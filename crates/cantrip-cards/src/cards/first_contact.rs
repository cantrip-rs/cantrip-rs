use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

/// Mill four cards (CR 701.17a), optionally take a permanent card (CR 110.4a, 701.17c), then gain 2 life (CR 119.3)
/// only when this spell is the first spell its controller cast this game (CR 601.2i): its own `Event::Cast` is that
/// player's earliest. Spells cast after it, in response, do not change that; a copy has no cast of its own (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_may_take_into_hand(g, item.controller, item.object, 4, "permanent", targets::is_permanent_card)?;
    if first_cast_this_game(g, item) {
        effects::gain_life(g, item, 2)?;
    }
    Ok(())
}

/// Whether `item`'s spell is the first its controller cast this game: the earliest of that player's casts in the history.
fn first_cast_this_game(g: &dyn Game, item: &StackItem) -> bool {
    let first = g.history().select(EventQuery::kind(EventMask::CAST).player(item.controller)).find_map(|r| match r.event {
        Event::Cast(c) => Some(c.spell),
        _ => None,
    });
    first == Some(item.object)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FIRST_CONTACT };
