use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, triggers};
use cantrip_oracle as oracle;

/// "Exile the top two cards of your library. You may play those cards this turn." (CR 406.3, 305.1, 514.2): a land among them
/// may be the turn's land drop.
fn exile_top_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_play_this_turn(g, item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_IF_CAST,
            effects::return_first_target_to_battlefield_tapped,
            "When Annie Flash enters, if you cast it, return target permanent card with mana value 3 or less from your graveyard to the battlefield tapped.",
        )
        .targets(&[targets::PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD]),
        triggered(
            triggers::THIS_BECOMES_TAPPED,
            exile_top_two,
            "Whenever Annie Flash becomes tapped, exile the top two cards of your library. You may play those cards this turn.",
        ),
    ],
    ..oracle::ANNIE_FLASH_THE_VETERAN
};
