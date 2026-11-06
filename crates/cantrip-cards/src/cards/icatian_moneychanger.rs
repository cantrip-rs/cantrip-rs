use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// This creature enters with three credit counters (CR 614.1c).
fn enters_with_credit(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let counters = &mut entering.counters[CounterKind::Credit as usize];
    *counters = counters.saturating_add(3);
    Ok(())
}

/// The entering creature deals damage to the trigger's controller (CR 113.7a).
fn damage_you(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, Target::Player(item.controller), 3);
    Ok(())
}

fn add_credit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Credit, 1);
    Ok(())
}

/// The sacrificed object's last-known counters determine the life gained (CR 608.2h).
fn gain_credit_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = g.obj(item.object).counters(CounterKind::Credit);
    g.gain_life(item.controller, amount);
    Ok(())
}

fn your_upkeep(q: &dyn Query, asker: &Asker) -> bool {
    q.state().turn.active == asker.controller && q.state().turn.step() == Step::Upkeep
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, damage_you, "When this creature enters, it deals 3 damage to you."),
        triggered(triggers::YOUR_UPKEEP, add_credit, "At the beginning of your upkeep, put a credit counter on this creature."),
        ability(SACRIFICE_THIS, gain_credit_life, "Sacrifice this creature: You gain 1 life for each credit counter on this creature. Activate only during your upkeep.")
            .condition(your_upkeep),
    ],
    enters: Some(enters_with_credit),
    ..oracle::ICATIAN_MONEYCHANGER
};
