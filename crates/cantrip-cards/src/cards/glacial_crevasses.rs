use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// A snow Mountain, with its land type read through this ability's text (CR 612.2).
fn snow_mountain(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).supertypes.snow
        && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Mountain))
}

fn prevent_combat_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::all_combat_damage_this_turn(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, snow_mountain),
        prevent_combat_damage,
        "Sacrifice a snow Mountain: Prevent all combat damage that would be dealt this turn.",
    )],
    granted_preventions: &[prevention::COMBAT_DAMAGE.text("Prevent all combat damage that would be dealt this turn.")],
    ..oracle::GLACIAL_CREVASSES
};
