use cantrip_api::*;
use cantrip_mechanics::{devotion, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you gain life equal to your devotion to green." (CR 700.5, 119.3): counted as the ability resolves.
fn gain_life_equal_devotion_green(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_0 = {
        let q: &dyn Query = g;
        i64::from(devotion::devotion(q, item.controller, Color::Green)) as i32
    };
    g.gain_life(item.controller,(amount_0) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            gain_life_equal_devotion_green,
            "When this creature enters, you gain life equal to your devotion to green.",
        ),
    ],
    ..oracle::NYLEAS_DISCIPLE
};
