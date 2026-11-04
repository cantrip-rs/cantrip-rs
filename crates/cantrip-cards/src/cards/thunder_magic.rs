use cantrip_api::*;
use cantrip_mechanics::{effects::damage_first_target, targets, tiered};
use cantrip_oracle as oracle;

const MODES: &[ModeDef] = &[
    mode(damage_first_target::<2>, "Thunder \u{2014} {0} \u{2014} Thunder Magic deals 2 damage to target creature.")
        .targets(&[targets::CREATURE])
        .cost(mana("{0}")),
    mode(damage_first_target::<4>, "Thundara \u{2014} {3} \u{2014} Thunder Magic deals 4 damage to target creature.")
        .targets(&[targets::CREATURE])
        .cost(mana("{3}")),
    mode(damage_first_target::<8>, "Thundaga \u{2014} {5}{R} \u{2014} Thunder Magic deals 8 damage to target creature.")
        .targets(&[targets::CREATURE])
        .cost(mana("{5}{R}")),
];

static MODAL: ModalDef = tiered::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::THUNDER_MAGIC };
