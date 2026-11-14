use cantrip_api::*;
use cantrip_mechanics::{counter_unless_pays_reporting, effects, targets};
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {2}. If they do, surveil 2" (CR 118.12, 701.6, 701.25): Don't Make a
/// Sound's controller surveils only when the spell's controller paid; if the spell was countered, or the target was no longer
/// legal, there is no surveil.
fn counter_unless_they_pay_two_then_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some((_, true)) = counter_unless_pays_reporting(g, item, 0, mana("{2}"))? {
        effects::surveil(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], counter_unless_they_pay_two_then_surveil), ..oracle::DONT_MAKE_A_SOUND };
