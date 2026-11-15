use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_opponents_creatures};
use cantrip_mechanics::{effects, your_turn};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(life(3), effects::this_connives, "Mental Organism \u{2014} Pay 3 life: M.O.D.O.K. connives. Activate only during your turn.").condition(your_turn),
    ],
    statics: &[static_ability(Layer::PtModify, your_opponents_creatures, plus_pt::<-1, -1>, "Designed Only for Killing \u{2014} Creatures your opponents control get -1/-1.")],
    ..oracle::M_O_D_O_K
};
