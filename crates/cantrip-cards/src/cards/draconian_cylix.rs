use cantrip_api::*;
use cantrip_mechanics::{regenerate, targets};
use cantrip_oracle as oracle;

/// "{2}, {T}, Discard a card at random: Regenerate target creature." (CR 608.2).
fn regenerate_target_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}")
                    .and(TAP)
                    .and(Cost {
                        discard_at_random: 1,
                        ..Cost::NONE
                    }),
                regenerate_target_creature,
                "{2}, {T}, Discard a card at random: Regenerate target creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::DRACONIAN_CYLIX
};
