use cantrip_api::*;
use cantrip_mechanics::effects::{self, gain_until_end_of_turn};
use cantrip_mechanics::triggers::YOU_GAIN_OR_LOSE_LIFE;
use cantrip_mechanics::your_turn;
use cantrip_oracle as oracle;

/// "Bats you control get +1/+0 and gain deathtouch until end of turn." (CR 611.2, 611.2c): the Bats its controller controls as it
/// resolves, this one among them, and none that enters later. Bat is a word of this card's text (CR 612.2).
fn bats_get_and_gain_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        for id in effects::subtype_you_control_get(g, item, Subtype::Bat, 1, 0) {
            gain_until_end_of_turn(g, item.object, id, Keyword::Deathtouch);
        }
    });
    Ok(())
}

/// "Whenever you gain or lose life during your turn" triggers once per life-gaining or life-losing event however much changed
/// (CR 603.2c), and "during your turn" is checked as the event happens (not an intervening "if", CR 603.4). "This ability triggers
/// only once each turn" is `triggers_each_turn(1)` (CR 603.2g).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_GAIN_OR_LOSE_LIFE,
        bats_get_and_gain_deathtouch,
        "Whenever you gain or lose life during your turn, Bats you control get +1/+0 and gain deathtouch until end of turn. This ability triggers only once each turn.",
    )
    .only_if(your_turn)
    .triggers_each_turn(1)],
    ..oracle::MOONSTONE_HARBINGER
};
