use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::creatures_you_control_get::<1, 1>, "Creatures you control get +1/+1 until end of turn."),
    mode(
        effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Hexproof]).0 }>,
        "Creatures you control gain hexproof until end of turn. (They can't be the targets of spells or abilities your opponents control.)",
    ),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::FAMILY_REUNION
};
