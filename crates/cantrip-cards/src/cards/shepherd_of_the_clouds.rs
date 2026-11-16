use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, permanents_with_subtype_you_control, targets};
use cantrip_oracle as oracle;

/// "Return target permanent card with mana value 3 or less from your graveyard to your hand. Return that card to the
/// battlefield instead if you control a Mount." Whether you control a Mount is checked as the ability resolves (CR 608.2c),
/// through the layers and in this ability's words (CR 612: a text-changing effect may change "Mount"); an opponent's Mount does
/// not count.
fn return_to_hand_or_battlefield_if_you_control_a_mount(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mount = g.words(item.object).subtype(Subtype::Mount);
    if permanents_with_subtype_you_control(g, item.controller, mount) > 0 {
        effects::return_first_target_to_battlefield(g, item)
    } else {
        effects::return_first_target_to_hand(g, item)
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        return_to_hand_or_battlefield_if_you_control_a_mount,
        "When this creature enters, return target permanent card with mana value 3 or less from your graveyard to your hand. Return that card to the battlefield instead if you control a Mount.",
    )
    .targets(&[targets::PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::SHEPHERD_OF_THE_CLOUDS
};
