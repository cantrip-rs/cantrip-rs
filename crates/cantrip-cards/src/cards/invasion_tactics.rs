use cantrip_api::*;
use cantrip_mechanics::triggers::{subtype_you_control_deal_combat_damage_to_player, THIS_ENTERS};
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::creatures_you_control_get::<2, 2>, "When this enchantment enters, creatures you control get +2/+2 until end of turn."),
        triggered(
            when_asked(EventMask::DAMAGE, subtype_you_control_deal_combat_damage_to_player::<{ lands::sub(Subtype::Ally) }>),
            |g, item| effects::draw(g, item, 1),
            "Whenever one or more Allies you control deal combat damage to a player, draw a card.",
        )
        .sums_per_subject(),
    ],
    ..oracle::INVASION_TACTICS
};
