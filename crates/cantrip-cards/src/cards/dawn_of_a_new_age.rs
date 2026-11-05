use cantrip_api::*;
use cantrip_oracle as oracle;

const HOPE: usize = CounterKind::Hope as usize;

/// "This enchantment enters with a hope counter on it for each creature you control" (CR 614.1c): counted as it
/// enters, among the permanents its controller-to-be controls, so it never counts itself.
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let creatures = g
        .permanents_of(entering.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .count();
    entering.counters[HOPE] += i64::from(creatures as i16);
    Ok(())
}

/// Remove a hope counter. If you do, draw a card. Then if it has no hope counters on it, sacrifice it and you gain
/// 4 life: the count read live while it is on the battlefield and as it last was once it is not (CR 608.2h), and the
/// life gain is not conditional on the sacrifice.
fn hope(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = item.object;
    if g.remove_counters(this, CounterKind::Hope, 1) > 0 {
        g.draw(item.controller, 1);
    }
    if g.obj(this).counters(CounterKind::Hope) > 0 {
        return Ok(());
    }
    g.sacrifice(item.controller, &[this]);
    g.gain_life(item.controller, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    abilities: &[triggered(cantrip_mechanics::triggers::YOUR_END_STEP, hope, "At the beginning of your end step, remove a hope counter from this enchantment. If you do, draw a card. Then if this enchantment has no hope counters on it, sacrifice it and you gain 4 life.")],
    ..oracle::DAWN_OF_A_NEW_AGE
};
