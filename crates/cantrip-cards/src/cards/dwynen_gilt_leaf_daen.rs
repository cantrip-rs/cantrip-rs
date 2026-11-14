use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

/// "Whenever Dwynen attacks, you gain 1 life for each attacking Elf you control." (CR 508.1, 608.2h, 119.3): counted as the ability resolves,
/// over every attacking creature under its controller's control with the creature type Elf (CR 612.2), Dwynen herself among them;
/// an Elf that stayed home or an attacking Elf under another player's control is not counted.
fn gain_life_for_each_attacking_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let attackers: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id).collect();
    let n = attackers.into_iter().filter(|&id| g.controller(id) == item.controller && g.chars(id).has_subtype(elf)).count();
    effects::gain_life(g, item, n as i32)
}

/// Reach is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        gain_life_for_each_attacking_elf,
        "Whenever Dwynen attacks, you gain 1 life for each attacking Elf you control.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>,
        statics::plus_pt::<1, 1>,
        "Other Elf creatures you control get +1/+1.",
    )],
    ..oracle::DWYNEN_GILT_LEAF_DAEN
};
