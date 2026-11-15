use cantrip_api::*;
use cantrip_mechanics::{delirium, fight, targets};
use cantrip_oracle as oracle;

/// "Choose target creature you control and target creature an opponent controls. Delirium - If there are four or more card
/// types among cards in your graveyard, put two +1/+1 counters on the creature you control. The creature you control deals
/// damage equal to its power to the creature an opponent controls." (CR 207.2c, 122.1, 120.3, 608.2b-c). Delirium is counted as the
/// spell resolves, the spell itself still on the stack; the counters come first, so the damage is dealt with the power they
/// give. A creature that is no longer a legal target gets no counters and deals or takes no damage.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if delirium(g, item.controller) {
        if let Some(creature) = targets::object(g, item, 0) {
            g.add_counters(creature, CounterKind::PlusOne, 2);
        }
    }
    g.end_sentence();
    fight::targets_bite(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], effect),
    ..oracle::BEASTIE_BEATDOWN
};
