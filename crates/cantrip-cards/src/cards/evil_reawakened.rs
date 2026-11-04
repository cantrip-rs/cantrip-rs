use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield with two additional +1/+1 counters on it" (CR 614.1c,
/// 122.6): the counters are put on as it enters, so it is never on the battlefield without them. With the target gone the spell
/// does not resolve (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield_with_counters(card, item.controller, false, &[(CounterKind::PlusOne, 2)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effect), ..oracle::EVIL_REAWAKENED };
