use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::each_opponent_discards_a_card_and_loses_life::<2>, "When this enchantment enters, each opponent discards a card and loses 2 life."),
        // Sacrificed or destroyed, it is looked back on (CR 603.10a) and scries from the graveyard.
        triggered(THIS_DIES, effects::scry_two, "When this enchantment is put into a graveyard from the battlefield, scry 2."),
        ability(cost("{2}{B}"), effects::sacrifice_self, "{2}{B}: Sacrifice this enchantment."),
    ],
    ..oracle::HOPELESS_NIGHTMARE
};
