use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cost_reduction::for_each_instant_and_sorcery_in_your_graveyard],
    abilities: &[triggered(triggers::THIS_ATTACKS, effects::may_cast_first_target_free_exiling_it, "Whenever The Dawning Archaic attacks, you may cast target instant or sorcery card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.")
        .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD])],
    ..oracle::THE_DAWNING_ARCHAIC
};
