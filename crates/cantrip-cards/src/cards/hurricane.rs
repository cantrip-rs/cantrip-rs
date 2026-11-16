use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Hurricane deals X damage to each creature with flying and each player." (CR 120.3, 608.2h).
fn hurricane(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_with_flying_and_each_player(g, item, item.x as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], hurricane),
    ..oracle::HURRICANE
};
