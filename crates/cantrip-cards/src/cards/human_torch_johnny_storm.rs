use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_DRAW_A_CARD;
use cantrip_mechanics::{effects, lands, power_up, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_DRAW_A_CARD, effects::damage_first_target::<1>, "Whenever you draw a card, if you control another Hero, Human Torch deals 1 damage to target opponent.")
            .targets(&[targets::OPPONENT])
            .condition(statics::you_control_another::<{ lands::sub(Subtype::Hero) }>),
        power_up::ability(
            cost("{6}{R}"),
            effects::three_plus_one_counters_on_this,
            "Power-up \u{2014} {6}{R}: Put three +1/+1 counters on Human Torch. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
        ),
    ],
    ..oracle::HUMAN_TORCH_JOHNNY_STORM
};
