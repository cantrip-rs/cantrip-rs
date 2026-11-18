use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

/// "Choose one. If you descended this turn, you may choose both instead." (CR 700.2, 700.11): checked as the modes are
/// chosen, as the spell is cast; both modes run in printed order (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target creature or planeswalker.").targets(&[targets::CREATURE_OR_PLANESWALKER]),
    mode(effects::destroy_first_target, "Destroy target noncreature, nonland permanent with mana value 1 or less.")
        .targets(&[target(targets::noncreature_nonland_permanent_with_mana_value_at_most::<1>)]),
])
.up_to_if(history::descended_this_turn, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::MOLTEN_COLLAPSE };
