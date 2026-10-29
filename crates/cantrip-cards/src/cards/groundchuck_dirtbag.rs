use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::TAPPED, mana::you_tap_a_land_for_mana),
        mana::that_player_adds_an_additional::<{ Mana::G as u8 }>,
        "Whenever you tap a land for mana, add {G}.",
    )
    .mana_ability(TriggeredMana::One(Mana::G))],
    ..oracle::GROUNDCHUCK_DIRTBAG
};
