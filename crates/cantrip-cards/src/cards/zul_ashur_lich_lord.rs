use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward_life::<2>("Ward\u{2014}Pay 2 life. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays 2 life.)"),
        ability(TAP, effects::may_cast_first_target_this_turn, "{T}: You may cast target Zombie creature card from your graveyard this turn.")
            .targets(&[targets::ZOMBIE_CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::ZUL_ASHUR_LICH_LORD
};
