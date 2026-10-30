use cantrip_api::*;
use cantrip_mechanics::{effects, mana, triggers};
use cantrip_oracle as oracle;

static WHEN_UNTAPPED: ModalDef = modes(&[
    mode(|g, item| effects::gain_life(g, item, 2), "You gain 2 life."),
    mode(|g, item| effects::surveil(g, item, 1), "Surveil 1."),
]);

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_unless_two_or_more_other_lands),
    abilities: &[
        modal_triggered(
            triggers::THIS_ENTERS_UNTAPPED,
            &WHEN_UNTAPPED,
            "When this land enters untapped, choose one — • You gain 2 life. • Surveil 1.",
        ),
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
    ],
    ..oracle::FEDERATION_OUTPOST
};
