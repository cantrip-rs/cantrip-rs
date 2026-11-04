use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Inferno deals 6 damage to each creature and each player." (CR 120.2).
fn inferno_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_and_player_n(g, item, 6);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], inferno_damage),
    ..oracle::INFERNO
};
