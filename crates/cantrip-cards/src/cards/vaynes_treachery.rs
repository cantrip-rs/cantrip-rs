use cantrip_api::*;
use cantrip_mechanics::{effects, kicker, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_minus_or_more_if_kicked::<2, 6>),
    // "Kicker—Sacrifice an artifact or creature." (CR 702.33a, 601.2b, 601.2h, 701.21): a kicker whose cost is not mana,
    // announced before the target and paid with the other costs; paying it kicks the spell once (CR 702.33d).
    optional_cost: Some(kicker::non_mana(sacrifice(1, targets::filters::artifact_or_creature))),
    labels: &["Cast without kicking", "Kicker\u{2014}Sacrifice an artifact or creature"],
    ..oracle::VAYNES_TREACHERY
};
