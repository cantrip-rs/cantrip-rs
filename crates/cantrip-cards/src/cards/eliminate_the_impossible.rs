use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Investigate. Creatures your opponents control get -2/-0 until end of turn. If any of them are suspected, they're no longer suspected."
/// (CR 701.16a, 611.2c, 701.60a): the creatures the opponents control as it resolves, no others; "them" is those creatures, so a
/// suspected creature of your own, or one that enters later, is left as it is. The -2/-0 and the unsuspecting are two sentences, neither
/// with an event, so a break ends the first; every creature the second names stops being suspected as one instruction.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)?;
    let creatures = effects::pump_creatures_your_opponents_control(g, item, -2, 0);
    g.end_sentence();
    instruction(g, |g| {
        for creature in creatures {
            if g.is_suspected(creature) {
                g.unsuspect(creature);
            }
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::ELIMINATE_THE_IMPOSSIBLE };
