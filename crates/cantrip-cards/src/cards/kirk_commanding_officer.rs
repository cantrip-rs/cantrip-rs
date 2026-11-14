use cantrip_api::*;
use cantrip_mechanics::effects::sacrifice_to_destroy_artifact_or_enchantment;
use cantrip_mechanics::{your_creatures_have_total_power_at_least, library, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ATTACK,
            library::exile_top_card_play_this_turn,
            "Whenever you attack while creatures you control have total power 8 or greater, exile the top card of your library. You may play that card this turn.",
        )
        .only_if(your_creatures_have_total_power_at_least::<8>),
        sacrifice_to_destroy_artifact_or_enchantment(2, "{2}, Sacrifice Kirk: Destroy target artifact or enchantment."),
    ],
    ..oracle::KIRK_COMMANDING_OFFICER
};
