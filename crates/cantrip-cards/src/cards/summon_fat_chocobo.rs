use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            tokens::create_a_landfall_bird,
            "I \u{2014} Wark \u{2014} Create a 2/2 green Bird creature token with \"Whenever a land you control enters, this token gets +1/+0 until end of turn.\"",
        ),
        chapter(2, effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>, "II, III, IV \u{2014} Kerplunk \u{2014} Creatures you control gain trample until end of turn."),
        chapter(3, effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>, "II, III, IV \u{2014} Kerplunk \u{2014} Creatures you control gain trample until end of turn."),
        chapter(4, effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>, "II, III, IV \u{2014} Kerplunk \u{2014} Creatures you control gain trample until end of turn."),
    ],
    ..oracle::SUMMON_FAT_CHOCOBO
};

pub static EXTRA: &[&CardDef] = &[&tokens::LANDFALL_BIRD];
