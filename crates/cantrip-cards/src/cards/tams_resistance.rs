use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn;
use cantrip_mechanics::{empower, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on up to one target creature. It gains vigilance until end of turn. Empower Jace 4."
fn counter_vigilance_and_empower(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    plus_one_counters_on_first_target_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Vigilance]).0 }>(g, item)?;
    empower::jace(g, item.controller, item.object, 4)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.optional()], counter_vigilance_and_empower),
    ..oracle::TAMS_RESISTANCE
};
