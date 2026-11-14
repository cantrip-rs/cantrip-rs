use cantrip_api::*;
use cantrip_mechanics::{your_creatures_have_total_power_at_least, effects, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature. Untap it."
fn counter_and_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.untap(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        counter_and_untap,
        "Whenever you attack while creatures you control have total power 8 or greater, put a +1/+1 counter on this creature. Untap it.",
    )
    .only_if(your_creatures_have_total_power_at_least::<8>)],
    ..oracle::TACTICAL_OFFICER
};
