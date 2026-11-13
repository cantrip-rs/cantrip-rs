use cantrip_api::*;
use cantrip_mechanics::{affinity, effects};
use cantrip_oracle as oracle;

/// "You draw three cards and gain 3 life." (CR 608.2c, 121.1, 119.3): one sentence, the cards drawn first and the life gained second.
fn draw_three_and_gain_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 3)?;
    effects::gain_life(g, item, 3)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifacts],
    spell: spell(&[], draw_three_and_gain_three),
    ..oracle::VOYAGE_HOME
};
