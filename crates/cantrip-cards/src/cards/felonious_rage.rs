use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, pump, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control gets +2/+0 and gains haste until end of turn. When that creature dies this turn, create a 2/2
/// white and blue Detective creature token." The pump and haste are one instruction (CR 608.2c); the delayed triggered
/// ability (CR 603.7b-c) watches that very object, so it triggers only if it dies this turn. A target no longer legal as this
/// resolves does nothing and sets up nothing (CR 608.2b).
fn boost_and_watch_for_its_death(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 2, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Haste);
    });
    delayed::this_turn(g, item, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], boost_and_watch_for_its_death),
    delayed: &[triggered(triggers::THIS_DIES, tokens::create_a_white_blue_detective, "When that creature dies this turn, create a 2/2 white and blue Detective creature token.")],
    ..oracle::FELONIOUS_RAGE
};
pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE];
