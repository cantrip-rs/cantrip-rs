use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// "You may have this creature enter as a copy of a creature or planeswalker you control, except it enters with an additional
/// +1/+1 counter on it if it's a creature, it enters with an additional loyalty counter on it if it's a planeswalker, and it
/// isn't legendary." (CR 614.1c, 614.12, 707.2, 707.9b, 122.6). "You" is the player it enters under (CR 110.2, 614.12), not its
/// owner, so the candidates are `entering.controller`'s permanents (`enter_as_copy_of_one_you_control`).
fn have_creature_enter_as_copy_enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    copy::enter_as_copy_of_one_you_control(g, entering, |c| c.is(CardType::Creature) || c.is(CardType::Planeswalker), |_, copy| copy.nonlegendary())?;
    if entering.copy.is_none() {
        return Ok(());
    }
    // "If it's a creature" and "if it's a planeswalker" are asked of it as it would enter, its copy chosen first (CR 614.12,
    // 616.1c; the 2019-05-03 ruling): a copy of Sol Ring under March of the Machines is a creature.
    let would = g.entering_chars(entering);
    if would.is(CardType::Creature) {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    if would.is(CardType::Planeswalker) {
        entering.counters[CounterKind::Loyalty as usize] += 1;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { enters_as_copy: Some(have_creature_enter_as_copy_enters), ..oracle::SPARK_DOUBLE };
