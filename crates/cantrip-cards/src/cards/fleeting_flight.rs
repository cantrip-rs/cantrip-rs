use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature. It gains flying until end of turn. Prevent all combat damage that would be dealt to it
/// this turn." (CR 122.1, 611.2a, 615.1): the counter and flying are the shared instruction, then, as a further one (CR 608.2c),
/// the prevention for the creature the spell targeted, if it is still a legal target (CR 608.2b). The prevention is the card's
/// first `granted_preventions` and is about that creature alone.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    plus_one_counters_on_first_target_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Flying]).0 }>(g, item)?;
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.end_sentence();
    prevention::this_turn(g, item, Target::Object(creature), 0, Shield::All);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_preventions: &[prevention::COMBAT_DAMAGE.text("Prevent all combat damage that would be dealt to it this turn.")],
    ..oracle::FLEETING_FLIGHT
};
