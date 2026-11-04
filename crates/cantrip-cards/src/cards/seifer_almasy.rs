use cantrip_api::*;
use cantrip_mechanics::triggers::{CREATURE_YOU_CONTROL_ATTACKS_ALONE, DEALS_COMBAT_DAMAGE_TO_PLAYER};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever a creature you control attacks alone, it gains double strike until end of turn" (CR 506.5, 702.4a, 611.2a): any
/// creature of its controller's, Seifer Almasy himself included, when it is the only creature declared as an attacker.
///
/// "Fire Cross \u{2014} Whenever Seifer Almasy deals combat damage to a player, you may cast target instant or sorcery card with
/// mana value 3 or less from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile
/// it instead" (CR 603.2, 118.9, 608.2g): see `effects::may_cast_first_target_free_exiling_it`.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            CREATURE_YOU_CONTROL_ATTACKS_ALONE,
            effects::subject_gains_double_strike_until_end_of_turn,
            "Whenever a creature you control attacks alone, it gains double strike until end of turn.",
        ),
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER,
            effects::may_cast_first_target_free_exiling_it,
            "Fire Cross \u{2014} Whenever Seifer Almasy deals combat damage to a player, you may cast target instant or sorcery card with mana value 3 or less from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
        )
        .targets(&[targets::INSTANT_OR_SORCERY_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::SEIFER_ALMASY
};
