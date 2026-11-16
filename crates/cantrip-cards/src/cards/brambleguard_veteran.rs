use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_oracle as oracle;

/// "Raccoons you control get +1/+1 and gain vigilance until end of turn." (CR 205.3, 611.2c): the Raccoons its controller
/// controls as the ability resolves, this creature among them while it is still one, and none that enters or becomes a
/// Raccoon later (the card's ruling), each getting +1/+1 and vigilance as one instruction. "Raccoons" is a word of this
/// card's text, read through it (CR 612.2).
fn pump_raccoons(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        for id in effects::subtype_you_control_get(g, item, Subtype::Raccoon, 1, 1) {
            effects::gain_until_end_of_turn(g, item.object, id, Keyword::Vigilance);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EXPEND_4,
        pump_raccoons,
        "Whenever you expend 4, Raccoons you control get +1/+1 and gain vigilance until end of turn. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
    )],
    ..oracle::BRAMBLEGUARD_VETERAN
};
