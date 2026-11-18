use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::this_dealt_damage;
use cantrip_oracle as oracle;

/// Investigate (CR 701.16a), made under the ability's controller.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// "Whenever this creature is dealt 3 or more damage, investigate" (CR 120.2, 603.2c, 701.16a): the damage it is dealt at once,
/// from every source that hurt it together (combat damage is simultaneous, CR 510.2), added up (`sums_at_least`); damage dealt
/// at different times does not add up (its ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::DAMAGE, this_dealt_damage),
        investigate,
        "Whenever this creature is dealt 3 or more damage, investigate.",
    )
    .sums_at_least(3)],
    ..oracle::INNOCENT_BYSTANDER
};
