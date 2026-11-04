use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {3}. If that spell is countered this way, exile it instead of putting
/// it into its owner's graveyard." (CR 118.12)
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::counter_unless_pays_else_exile(g, item, 0, mana("{3}"))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effect),
    ..oracle::NO_MORE_LIES
};
