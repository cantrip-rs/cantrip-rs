use cantrip_api::*;
use cantrip_mechanics::library::exile_top_card_play_until_end_of_next_turn;
use cantrip_mechanics::{effects, outlaw};
use cantrip_oracle as oracle;

/// "Creatures you control get +2/+0 until end of turn. If you control an outlaw, exile the top card of your library. Until the end of
/// your next turn, you may play that card." The pump goes first and affects only the creatures there as it resolves (CR 611.2c); the
/// "if" is checked as the sentence is reached, after the pump (CR 608.2c), by what the controller then controls (CR 700.12a).
fn fury(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 2, 0);
    g.end_sentence();
    if outlaw::outlaws_controlled_by(g, item.controller) > 0 {
        exile_top_card_play_until_end_of_next_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], fury),
    ..oracle::OUTLAWS_FURY
};
