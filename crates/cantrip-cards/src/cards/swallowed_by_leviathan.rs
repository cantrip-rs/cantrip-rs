use cantrip_api::*;
use cantrip_mechanics::{counter_unless_pays, effects, targets};
use cantrip_oracle as oracle;

/// "Choose target spell. Surveil 2, then counter the chosen spell unless its controller pays {1} for each card in your
/// graveyard" (CR 701.25a, 118.12): the surveil is finished, the cards it put there in the graveyard, before the cards are
/// counted, and this spell is on the stack, not in the graveyard, as it resolves (CR 608.2n). The payment is asked even when
/// it is {0} (CR 118.5). Nothing happens, the surveil included, if the target is no longer legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)?;
    let cards = g.state().zone(Zone::graveyard(item.controller)).len() as u32;
    counter_unless_pays(g, item, 0, generic(cards))?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], effect), ..oracle::SWALLOWED_BY_LEVIATHAN };
