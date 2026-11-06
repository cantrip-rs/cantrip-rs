use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Whenever a player casts a black spell, Sol'kanar’s controller gains 1 life (CR 603.2, 119.3).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::a_player_casts_a_spell_of_color::<{ Color::Black as u8 }>),
        effects::gain_one_life,
        "Whenever a player casts a black spell, you gain 1 life.",
    )],
    ..oracle::SOLKANAR_THE_SWAMP_KING
};
