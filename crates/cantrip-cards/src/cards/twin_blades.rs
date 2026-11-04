use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target_and_gain_until_end_of_turn, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Attach it to target creature you control. That creature gains double strike until end of turn."
fn attach_and_double_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    attach_to_target_and_gain_until_end_of_turn(g, item, Keyword::DoubleStrike)
}

/// Flash is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            attach_and_double_strike,
            "When this Equipment enters, attach it to target creature you control. That creature gains double strike until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::TWIN_BLADES
};
