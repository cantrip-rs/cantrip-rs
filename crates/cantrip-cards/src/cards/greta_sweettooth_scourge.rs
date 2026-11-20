use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When Greta enters, create a Food token."),
        // The Food is part of each cost (CR 602.2b), and one Food cannot pay two costs (the rulings).
        ability(
            cost("{G}").and(Cost::sacrifice(1, filters::food)),
            effects::plus_one_counter_on_target,
            "{G}, Sacrifice a Food: Put a +1/+1 counter on target creature. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE])
        .sorcery_speed(),
        ability(
            cost("{1}{B}").and(Cost::sacrifice(1, filters::food)),
            effects::draw_a_card_and_lose_1_life,
            "{1}{B}, Sacrifice a Food: You draw a card and you lose 1 life.",
        ),
    ],
    ..oracle::GRETA_SWEETTOOTH_SCOURGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
