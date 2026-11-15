use cantrip_api::*;
use cantrip_mechanics::effects::this_gets;
use cantrip_mechanics::triggers::YOU_GAIN_OR_LOSE_LIFE;
use cantrip_mechanics::your_turn;
use cantrip_oracle as oracle;

/// "Whenever you gain or lose life during your turn" triggers once per life-gaining or life-losing event however much changed
/// (CR 603.2c, the rulings): the shared life-change trigger fires per event, and "during your turn" is checked as the event
/// happens (CR 603.4 does not apply, it is not an intervening "if").
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_OR_LOSE_LIFE, this_gets::<1, 0>, "Whenever you gain or lose life during your turn, this creature gets +1/+0 until end of turn.").only_if(your_turn)],
    ..oracle::WAX_WANE_WITNESS
};
