use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// The cost can sacrifice a land with the snow supertype (CR 205.4g, 701.21a).
fn snow_land(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).supertypes.snow && q.chars(id).is(CardType::Land)
}

/// Prevent all combat damage through this turn (CR 615.1, 611.2a).
fn prevent_combat_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::all_combat_damage_this_turn(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}").and(sacrifice(1, snow_land)),
        prevent_combat_damage,
        "{2}, Sacrifice a snow land: Prevent all combat damage that would be dealt this turn.",
    )],
    granted_preventions: &[prevention::COMBAT_DAMAGE.text("Prevent all combat damage that would be dealt this turn.")],
    ..oracle::SUNSTONE
};
