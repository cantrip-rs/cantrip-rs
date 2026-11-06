use cantrip_api::*;
use cantrip_mechanics::{triggers, cumulative_upkeep, effects};
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_oracle as oracle;

/// "When this enchantment leaves the battlefield, you lose 20 life." (CR 608.2, 119.3).
fn lose_20_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 20);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{2}")), "Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"),
        triggered(
            triggers::THIS_ENTERS,
            |g, item| effects::gain_life(g, item, 20),
            "When this enchantment enters, you gain 20 life.",
        ),
        triggered(
            triggers::THIS_LEAVES,
            lose_20_life,
            "When this enchantment leaves the battlefield, you lose 20 life.",
        ),
    ],
    ..oracle::ILLUSIONS_OF_GRANDEUR
};
