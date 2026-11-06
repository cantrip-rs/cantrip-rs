use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::EACH_END_STEP};
use cantrip_oracle as oracle;

/// The intervening condition is checked at triggering and resolution (CR 603.4).
fn no_creatures(q: &dyn Query, _: &Asker) -> bool {
    !q.state().battlefield().iter().any(|&id| q.chars(id).is(CardType::Creature))
}

/// Deal the damage simultaneously (CR 120.3).
fn damage_each(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_and_player_n(g, item, 1);
    Ok(())
}

/// "{B}: This enchantment deals 1 damage to each creature and each player. Activate no more times each turn than the number of snow Swamps you control." (CR 608.2).
fn enchantment_deals_1_damage_creature_more_this_turn(
    q: &dyn Query,
    asker: &Asker,
) -> bool {
    let made = q
        .history()
        .select(EventQuery::kind(EventMask::ABILITY_ACTIVATED).this_turn())
        .filter(|r| {
            matches!(
                r.event, Event::AbilityActivated { source, ability, .. } if source ==
                asker.source && q.activated_ability(ability).is_some_and(| def | def.text
                ==
                "{B}: This enchantment deals 1 damage to each creature and each player. Activate no more times each turn than the number of snow Swamps you control.")
            )
        })
        .count() as i64;
    made
        < i64::from(
            q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).supertypes.snow
                        && q
                            .chars(id)
                            .has_subtype(q.words(asker.source).subtype(Subtype::Swamp))
                        && asker.controller == q.controller(id)
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64,
        )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                EACH_END_STEP,
                effects::sacrifice_self,
                "At the beginning of the end step, if no creatures are on the battlefield, sacrifice this enchantment.",
            )
            .condition(no_creatures),
        ability(
                cost("{B}"),
                damage_each,
                "{B}: This enchantment deals 1 damage to each creature and each player. Activate no more times each turn than the number of snow Swamps you control.",
            )
            .condition(enchantment_deals_1_damage_creature_more_this_turn),
    ],
    ..oracle::WITHERING_WISPS
};
