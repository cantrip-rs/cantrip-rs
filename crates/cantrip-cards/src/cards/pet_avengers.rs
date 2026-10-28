use cantrip_api::*;
use cantrip_mechanics::{power_up, tokens};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature and create a 3/2 white Hero creature token with vigilance."
fn counter_and_hero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    tokens::create(g, item.controller, &tokens::HERO_3_2_WHITE_VIGILANCE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{6}{G}"),
        counter_and_hero,
        "Power-up \u{2014} {6}{G}: Put a +1/+1 counter on this creature and create a 3/2 white Hero creature token with vigilance. (Activate each power-up ability only once. Reduce the cost by its mana cost if it entered this turn.)",
    )],
    ..oracle::PET_AVENGERS
};

pub static EXTRA: &[&CardDef] = &[&tokens::HERO_3_2_WHITE_VIGILANCE];
