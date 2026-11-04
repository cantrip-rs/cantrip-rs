use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "Prevent the next 1 damage that would be dealt to you this turn" (CR 615.7).
fn prevent_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::this_turn(g, item, Target::Player(item.controller), 0, Shield::Next(1));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}"), prevent_damage, "{2}: Prevent the next 1 damage that would be dealt to you this turn.")],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents up to 1 damage",
        ..prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to you this turn.")
    }],
    ..oracle::SHIELD_OF_THE_AGES
};
