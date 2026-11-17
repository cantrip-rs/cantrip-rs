use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control gains your choice of menace or lifelink until end of turn." The choice is made as the ability
/// resolves (CR 608.2d), so it is not asked of a target that is no longer legal (CR 608.2b).
fn gain_choice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_your_choice_until_end_of_turn(g, item, &CARD, creature, &[Keyword::Menace, Keyword::Lifelink])
}

/// Deathtouch is a core keyword. "Whenever you commit a crime" (CR 700.13) triggers for each crime, so each gives a creature
/// one keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_COMMIT_A_CRIME,
        gain_choice,
        "Whenever you commit a crime, target creature you control gains your choice of menace or lifelink until end of turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    labels: &["Menace", "Lifelink"],
    ..oracle::RATTLEBACK_APOTHECARY
};
