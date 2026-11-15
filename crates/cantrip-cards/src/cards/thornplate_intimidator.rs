use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, offspring, targets};
use cantrip_oracle as oracle;

/// "Target opponent loses 3 life unless they sacrifice a nonland permanent of their choice or discard a card." (CR 608.2d, 119.3, 701.9,
/// 701.21a): the target opponent chooses as the ability resolves to sacrifice a nonland permanent, to discard a card, or to lose 3 life, and
/// always may lose the life, even holding cards and permanents (the rulings); with neither to give they lose it. The one choice is done
/// as one action (`Game::together`). With the target gone the ability does nothing (CR 608.2b).
fn target_opponent_loses_3_life_unless_they_sacrifice_a_nonland_permanent_or_discard_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let (mut sacrificed, mut discarded, mut losses) = (Vec::new(), Vec::new(), Vec::new());
    effects::choose_sacrifice_nonland_or_discard_or_lose_life(g, item, opponent, 3, &mut sacrificed, &mut discarded, &mut losses)?;
    g.together(Together { sacrifices: &sacrificed, discards: &discarded, life: &losses, ..Together::default() });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{3}"))),
    abilities: &[
        triggered(
            THIS_ENTERS,
            target_opponent_loses_3_life_unless_they_sacrifice_a_nonland_permanent_or_discard_a_card,
            "When this creature enters, target opponent loses 3 life unless they sacrifice a nonland permanent of their choice or discard a card.",
        )
        .targets(&[targets::OPPONENT]),
        offspring::trigger("Offspring {3} (You may pay an additional {3} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
    ],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Lose 3 life"],
    ..oracle::THORNPLATE_INTIMIDATOR
};
