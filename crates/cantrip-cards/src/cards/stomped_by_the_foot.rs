use cantrip_api::*;
use cantrip_mechanics::{effects, kicker, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Kicker - Sacrifice an artifact or creature" is a kicker (CR 702.33a) whose cost is not mana, so it is announced before the
    // target (CR 601.2b) and paying it kicks the spell once (CR 702.33d): `kicked`, as for any kicker.
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_minus_or_more_if_kicked::<2, 5>),
    optional_cost: Some(kicker::non_mana(sacrifice(1, targets::filters::artifact_or_creature))),
    labels: &["Cast without kicking", "Kicker\u{2014}Sacrifice an artifact or creature"],
    ..oracle::STOMPED_BY_THE_FOOT
};
