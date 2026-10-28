use cantrip_api::*;
use cantrip_mechanics::{effects, recruit, targets, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, recruit::effect, "I \u{2014} Recruit."),
        chapter(2, effects::return_first_target_to_battlefield, "II \u{2014} Return target creature card with mana value 3 or less from your graveyard to the battlefield.")
            .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD]),
        chapter(3, effects::plus_one_counter_on_target, "III \u{2014} Put a +1/+1 counter on up to one target creature.").targets(&[targets::CREATURE.optional()]),
    ],
    ..oracle::THE_MOUNTAIN_KINGS_RETURN
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
