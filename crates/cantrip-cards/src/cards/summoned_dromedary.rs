use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{1}{W}: Return this card from your graveyard to your hand. Activate only as a sorcery." An activated ability that
/// functions only in its owner's graveyard (CR 113.6m), which `from` says.
fn return_from_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_this_to_hand(g, item)
}

/// Vigilance is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{W}"), return_from_graveyard, "{1}{W}: Return this card from your graveyard to your hand. Activate only as a sorcery.")
        .from(ZoneKind::Graveyard)
        .sorcery_speed()],
    ..oracle::SUMMONED_DROMEDARY
};
