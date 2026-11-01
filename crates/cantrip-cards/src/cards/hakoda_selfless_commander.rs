use cantrip_api::*;
use cantrip_mechanics::effects::pump_creatures_you_control_and_gain;
use cantrip_mechanics::{lands, statics::subtype_spell};
use cantrip_oracle as oracle;

/// "Creatures you control get +0/+5 and gain indestructible until end of turn." (CR 611.2c): the creatures its controller
/// controls as it resolves, Hakoda himself gone as a cost.
fn creatures_get_toughness_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_creatures_you_control_and_gain(g, item, 0, 5, Keyword::Indestructible);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2), and "you may cast Ally spells from the top of your
    // library" (CR 601.3).
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopSpells(subtype_spell::<{ lands::sub(Subtype::Ally) }>, None)],
    abilities: &[ability(
        SACRIFICE_THIS,
        creatures_get_toughness_and_indestructible,
        "Sacrifice Hakoda: Creatures you control get +0/+5 and gain indestructible until end of turn.",
    )],
    ..oracle::HAKODA_SELFLESS_COMMANDER
};
