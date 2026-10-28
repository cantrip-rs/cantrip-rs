use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_oracle as oracle;

/// Gau deals damage equal to its power to each opponent: its power as the ability resolves, or as it last was.
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or(0).max(0) as u16;
    cantrip_mechanics::effects::damage_each_opponent(g, item, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::THIS_ATTACKS, plus_one_counter_on_this, "Rage \u{2014} Whenever Gau attacks, put a +1/+1 counter on it."),
        triggered(cantrip_mechanics::triggers::EACH_END_STEP, damage, "At the beginning of each end step, if a card left your graveyard this turn, Gau deals damage equal to its power to each opponent.")
            .condition(cantrip_mechanics::history::a_card_left_your_graveyard_this_turn),
    ],
    ..oracle::GAU_FERAL_YOUTH
};
