use cantrip_api::*;
use cantrip_mechanics::{effects, mana, triggers::THIS_ENTERS_UNTAPPED};
use cantrip_oracle as oracle;

/// "When this land enters untapped, choose one —" (CR 603.6a, 700.2b).
static MODAL: ModalDef = modes(&[
    mode(effects::damage_each_opponent_n::<1>, "This land deals 1 damage to each opponent."),
    mode(|g, item| effects::surveil(g, item, 1), "Surveil 1."),
]);

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_unless_two_or_more_other_lands),
    abilities: &[
        modal_triggered(
            THIS_ENTERS_UNTAPPED,
            &MODAL,
            "When this land enters untapped, choose one —\n• This land deals 1 damage to each opponent.\n• Surveil 1.",
        ),
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
    ],
    ..oracle::QUARKS_BAR
};
