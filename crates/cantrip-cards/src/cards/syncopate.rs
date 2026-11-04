use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const EXILED: Question =
    Question::new("Countered", "pay {cost}, or {subject} is countered and exiled?", "", "Pay {cost}", "Let it be countered");

/// "Counter target spell unless its controller pays {X}. If that spell is countered this way, exile it instead of putting
/// it into its owner's graveyard" (CR 118.12, 107.3m): X is the value chosen as this was cast, and the payment is asked
/// even when it is {0} (CR 118.5).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let payer = g.controller(spell);
    if !g.may_pay(payer, item.object, spell, Cost { mana: generic(item.x as u32), ..Cost::NONE }, &EXILED)? {
        g.counter_to(spell, CounterTo::Exile);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], effect), ..oracle::SYNCOPATE };
