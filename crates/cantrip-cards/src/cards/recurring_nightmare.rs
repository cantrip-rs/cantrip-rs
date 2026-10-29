use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(RETURN_THIS.and(sacrifice(1, filters::creature)), effects::return_target_card_to_battlefield, "Sacrifice a creature, Return this enchantment to its owner's hand: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[cantrip_mechanics::targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::RECURRING_NIGHTMARE
};
