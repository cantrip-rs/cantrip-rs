use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Grant the landwalk named by this ability's current text (CR 612.2, 702.14).
fn gains_landwalk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let subtype = g.words(item.object).subtype(Subtype::Mountain);
    let Some(walk) = Keyword::landwalk(subtype) else { return Ok(()) };
    effects::this_gains_until_end_of_turn(g, item, walk)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{U}"), gains_landwalk, "{U}: This creature gains mountainwalk until end of turn.")],
    ..oracle::RIVER_MERFOLK
};
