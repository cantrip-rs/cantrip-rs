use cantrip_api::*;
use cantrip_mechanics::{cast_from_your_graveyard, replacements};
use cantrip_oracle as oracle;


pub static CARD: CardDef = CardDef {
    granted_ways: &[GrantedWay {
        text: "You may cast artifact spells from your graveyard by paying 3 life in addition to paying their other costs.",
        cost: cast_from_your_graveyard::card_of_type::<{ CardType::Artifact as u8 }>,
        permission: true,
        additional: life(3),
        spell_replacement: Some(0),
        ..GrantedWay::CAST
    }],
    // "If you cast a spell this way, that artifact enters with a finality counter on it": a replacement effect about the spell
    // cast by the way (`GrantedWay::spell_replacement`), made as it is cast and so still applying if Noctis has left before it
    // resolves.
    granted_replacements: &[replacements::SPELL_ENTERS_WITH_FINALITY
        .text("If you cast a spell this way, that artifact enters with a finality counter on it.")],
    ..oracle::NOCTIS_PRINCE_OF_LUCIS
};
