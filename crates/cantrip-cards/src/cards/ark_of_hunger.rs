use cantrip_api::*;
use cantrip_mechanics::{effects, library, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more cards leave your graveyard, this artifact deals 1 damage to each opponent and you gain 1 life": one
/// trigger for the cards that left together (CR 603.2c), the damage and the life one effect (CR 608.2c).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::CARDS_LEAVE_YOUR_GRAVEYARD,
            drain,
            "Whenever one or more cards leave your graveyard, this artifact deals 1 damage to each opponent and you gain 1 life.",
        )
        .once_per_batch(),
        ability(TAP, library::mill_a_card_and_play_it_this_turn, "{T}: Mill a card. You may play that card this turn."),
    ],
    ..oracle::ARK_OF_HUNGER
};
