use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Other creatures you control get +2/+2 and gain vigilance and menace until end of turn. Damage can't be prevented this
/// turn." (CR 611.2c, 615.12): the creatures its controller controls as it resolves but this one, and none that enters later;
/// then, a second sentence, the rule of the game until the turn ends.
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_other_creatures_you_control_and_gain(g, item, 2, 2, &[Keyword::Vigilance, Keyword::Menace]);
    g.end_sentence();
    g.damage_cant_be_prevented();
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        rally,
        "When this creature enters, other creatures you control get +2/+2 and gain vigilance and menace until end of turn. Damage can't be prevented this turn.",
    )],
    ..oracle::PYREWOOD_GEARHULK
};
