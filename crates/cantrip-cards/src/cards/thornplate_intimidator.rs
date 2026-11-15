use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{offspring, players, targets};
use cantrip_oracle as oracle;

/// "Target opponent loses 3 life unless they sacrifice a nonland permanent of their choice or discard a card." (CR 608.2d, 119.3, 701.9,
/// 701.21a): the target opponent chooses as the ability resolves to sacrifice a nonland permanent, to discard a card, or to lose 3 life, and
/// always may lose the life, even holding cards and permanents (the rulings); with neither to give they lose it. With the target gone the
/// ability does nothing (CR 608.2b). `players::target_player_loses_life_unless_sacrifices_or_discards`, with the labels below.
pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{3}"))),
    abilities: &[
        triggered(
            THIS_ENTERS,
            players::target_player_loses_life_unless_sacrifices_or_discards::<3>,
            "When this creature enters, target opponent loses 3 life unless they sacrifice a nonland permanent of their choice or discard a card.",
        )
        .targets(&[targets::OPPONENT]),
        offspring::trigger("Offspring {3} (You may pay an additional {3} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
    ],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Lose 3 life"],
    ..oracle::THORNPLATE_INTIMIDATOR
};
