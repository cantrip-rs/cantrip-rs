use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "X target creatures gain islandwalk until end of turn. (They can't be blocked as long as defending player controls an Island.)" (CR 608.2, 611.2a).
fn x_target_creatures_gain_islandwalk(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::legal_objects(g, item);
    let Some(walk) = Keyword::landwalk(g.words(item.object).subtype(Subtype::Island)) else { return Ok(()) };
    let kind_0_0_0 = EffectKind::AddKeywords(Keywords::of(&[walk]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.counted()], x_target_creatures_gain_islandwalk),
    ..oracle::PART_WATER
};
