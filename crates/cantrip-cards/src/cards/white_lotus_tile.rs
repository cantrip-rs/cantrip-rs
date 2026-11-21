use cantrip_api::*;
use cantrip_mechanics::mana::enters_tapped;
use cantrip_oracle as oracle;

/// The greatest number of creatures `player` controls that have one creature type in common (CR 205.3m, 702.73a): each creature
/// type is counted among the creatures that have it as they are now, a changeling (every creature type) in each, and the
/// largest count wins (the ruling of 2025-11-14: always the greatest number possible). None means 0.
fn greatest_number_sharing_a_creature_type(q: &dyn Query, player: PlayerId) -> u32 {
    let creatures: Vec<SubtypeSet> =
        q.permanents_of(player).into_iter().filter(|&id| q.chars(id).is(CardType::Creature)).map(|id| q.chars(id).subtypes).collect();
    Subtype::CREATURE_TYPES.iter().map(|&t| creatures.iter().filter(|s| s.has(t)).count() as u32).fold(0, u32::max)
}

/// "{T}: Add X mana of any one color, where X is the greatest number of creatures you control that have a creature type in common"
/// (CR 605.1a, 106.3, 106.5): the color chosen as it is activated, X as it resolves (a mana ability resolves at once).
fn add_x_of_the_chosen_color(g: &mut dyn Game, asker: &Asker, chosen: Option<Color>) -> Result<(), Suspend> {
    let Some(chosen) = chosen else { return Ok(()) };
    let x = greatest_number_sharing_a_creature_type(g, asker.controller);
    g.add_mana(asker.controller, chosen.mana(), x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped),
    abilities: &[mana_ability(
        TAP,
        add_by(add_x_of_the_chosen_color).choose_color(),
        "{T}: Add X mana of any one color, where X is the greatest number of creatures you control that have a creature type in common.",
    )],
    ..oracle::WHITE_LOTUS_TILE
};
