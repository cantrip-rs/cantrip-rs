use cantrip_api::*;
use cantrip_mechanics::{lands, permanents_with_subtype_you_control, statics};
use cantrip_oracle as oracle;

/// "{T}: Add {G} for each Elf you control." (CR 106.4, 605.1a): a mana ability, counted as it resolves over the permanents its
/// controller controls with the creature type Elf (CR 612.2), the Archdruid itself among them; an opponent's Elf does not count.
fn add_green_for_each_elf_you_control(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let elf = g.words(asker.source).subtype(Subtype::Elf);
    let elves = permanents_with_subtype_you_control(g, asker.controller, elf);
    if elves > 0 {
        g.add_mana(asker.controller, Mana::G, elves as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_by(add_green_for_each_elf_you_control), "{T}: Add {G} for each Elf you control.")],
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>,
        statics::plus_pt::<1, 1>,
        "Other Elf creatures you control get +1/+1.",
    )],
    ..oracle::ELVISH_ARCHDRUID
};
