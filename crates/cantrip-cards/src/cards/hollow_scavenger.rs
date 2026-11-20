use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Bakery Raid.
pub static BAKERY_RAID: CardDef = CardDef { spell: spell(&[], tokens::create_a_food_token), ..oracle::BAKERY_RAID };

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(Cost::sacrifice(1, filters::food)),
        effects::pump_self::<2, 2>,
        "{1}, Sacrifice a Food: This creature gets +2/+2 until end of turn. Activate only once each turn.",
    )
    .once_each_turn()],
    adventure: Some(&BAKERY_RAID),
    ..oracle::HOLLOW_SCAVENGER
};

pub static EXTRA: &[&CardDef] = &[&BAKERY_RAID, &tokens::FOOD];
