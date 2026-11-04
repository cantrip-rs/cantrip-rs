use cantrip_api::*;
use cantrip_mechanics::library::exile_top_play_until_your_next_end_step;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+1 and gains haste until end of turn. Exile the top card of your library. You may play it until your
/// next end step." (CR 611.2a, 702.10, 608.2c, 305.1): the pump first, then the exile, each as its own sentence; the card may be
/// played, a land as the turn's land drop, until your next end step begins. With the creature gone as it resolves the spell does
/// nothing at all (CR 608.2b).
fn pump_then_exile_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_haste::<3, 1>(g, item)?;
    exile_top_play_until_your_next_end_step(g, item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], pump_then_exile_the_top_card), ..oracle::HASTE_MAGIC };
