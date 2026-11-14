use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets};
use cantrip_oracle as oracle;

/// "Activate only if you have at least 10 life more than your starting life total" (CR 602.5): asked of the game state each time the
/// ability is offered, and once it is activated nothing that happens to the life total matters (the ruling of 2024-11-08).
fn ten_life_above_your_starting_life(q: &dyn Query, asker: &Asker) -> bool {
    statics::player_life_above_starting_life(q.state(), asker.controller) >= 10
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(sacrifice(1, targets::filters::another_creature)),
            effects::gain_life_equal_sacrificed_creatures_toughness,
            "{1}, Sacrifice another creature: You gain life equal to the sacrificed creature's toughness.",
        ),
        ability(
            cost("{1}{W}{B}").and(sacrifice(1, targets::filters::another_creature)),
            effects::exile_first_target,
            "{1}{W}{B}, Sacrifice another creature: Exile target nonland permanent. Activate only if you have at least 10 life more than your starting life total.",
        )
        .targets(&[targets::NONLAND_PERMANENT])
        .condition(ten_life_above_your_starting_life),
    ],
    ..oracle::AYLI_ETERNAL_PILGRIM
};
