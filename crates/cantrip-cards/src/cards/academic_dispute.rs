use cantrip_api::*;
use cantrip_mechanics::effects::{self, ask_may};
use cantrip_mechanics::{learn, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("GainReach", "have the creature gain reach until end of turn?", "Chosen as Academic Dispute resolves", "Gain reach", "Don't");

/// "Target creature blocks this turn if able. You may have it gain reach until end of turn. Learn." The creature must block
/// if able (CR 509.1c); the reach is chosen as the spell resolves (ruling 2021-04-16). Its one target illegal, the spell does
/// not resolve at all (CR 608.2b), so there is always a creature here.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_blocks_this_turn_if_able(g, item)?;
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.end_sentence();
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Reach);
    }
    g.end_sentence();
    learn::learn(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::ACADEMIC_DISPUTE
};
