use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {X}. If that spell is countered this way, exile it instead of putting
/// it into its owner's graveyard" (CR 118.12, 107.3m): X is the value chosen as this was cast, and the payment is asked
/// even when it is {0} (CR 118.5).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::counter_unless_pays_else_exile(g, item, 0, generic(item.x as u32))
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], effect), ..oracle::SYNCOPATE };
