use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{permanents_with_subtype_you_control, pump, subtype_cards_in_graveyard, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +X/+0 until end of turn and up to one target creature an opponent controls gets -0/-X until
/// end of turn, where X is the number of Elves you control plus the number of Elf cards in your graveyard" (CR 611.2a, 608.2h):
/// X is worked out once, as the ability resolves, and both creatures get it; a target that is no longer legal is left alone
/// (CR 608.2b).
fn elves_pump_and_shrink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let x = (permanents_with_subtype_you_control(g, item.controller, elf) + subtype_cards_in_graveyard(g, item.controller, elf)) as i16;
    // One instruction (`Game::one_instruction`): the two "gets" are one sentence, joined by "and".
    g.one_instruction(&mut |g| {
        if let Some(mine) = targets::object(g, item, 0) {
            pump(g, item.object, mine, x, 0);
        }
        if let Some(theirs) = targets::object(g, item, 1) {
            pump(g, item.object, theirs, 0, -x);
        }
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        elves_pump_and_shrink,
        "When this creature enters, target creature you control gets +X/+0 until end of turn and up to one target creature an opponent controls gets -0/-X until end of turn, where X is the number of Elves you control plus the number of Elf cards in your graveyard.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS.optional()])],
    ..oracle::GLOOM_RIPPER
};
