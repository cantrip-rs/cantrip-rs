use cantrip_api::*;
use cantrip_mechanics::{behold, effects, targets};
use cantrip_oracle as oracle;

/// "You may cast this spell as though it had flash if you behold a Dragon as an additional cost to cast it" (CR 601.2b, 701.4a,
/// 118.8): the flash permission of `OptionalCost::flash`, bought with the behold cost the Exhales print (`behold::optional`)
/// instead of mana. Where only the permission lets it be cast, the Dragon is chosen or revealed as the cost is paid, unasked;
/// at sorcery speed beholding is optional, as any optional cost is.
pub static CARD: CardDef = CardDef {
    optional_cost: Some(OptionalCost { flash: true, ..behold::optional(behold::dragon, "Dragon") }),
    labels: &["Cast without flash", "Flash: behold a Dragon (choose a Dragon you control or reveal one from your hand)"],
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effects::damage_first_target::<4>),
    ..oracle::MOLTEN_EXHALE
};
