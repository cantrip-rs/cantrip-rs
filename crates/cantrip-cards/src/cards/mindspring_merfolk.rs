use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw X cards. Put a +1/+1 counter on each Merfolk creature you control." (CR 121.1, 122.1, 107.3): X as the ability was
/// activated (zero draws nothing, the counters still go on); the Merfolk judged as it resolves, so one that entered or left
/// meanwhile counts as it then stands (CR 608.2), the source among them if it is still a Merfolk creature on the battlefield.
fn draw_x_and_counter_on_each_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, item.x)?;
    effects::counters_on_each_creature_of_subtype_you_control(g, item, Subtype::Merfolk, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{X}{U}{U}, {T}"),
        draw_x_and_counter_on_each_merfolk,
        "Exhaust \u{2014} {X}{U}{U}, {T}: Draw X cards. Put a +1/+1 counter on each Merfolk creature you control. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::MINDSPRING_MERFOLK
};
