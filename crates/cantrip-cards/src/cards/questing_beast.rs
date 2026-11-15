use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers};
use cantrip_oracle as oracle;

/// "Combat damage that would be dealt by creatures you control can't be prevented" (CR 615.12): the damage of any
/// creature its controller controls, the Beast included, and only combat damage.
fn creatures_you_control_deal_unpreventable_combat_damage(q: &dyn Query, asker: &Asker, source: ObjectId, _: Target, combat: bool) -> bool {
    combat && q.controller(source) == asker.controller && q.chars(source).is(CardType::Creature)
}

/// "Target planeswalker that player controls", the player the damage was dealt to being the event's subject.
fn planeswalker_that_player_controls(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::controlled_by_that_player(q, id) && q.chars(id).is(CardType::Planeswalker))
}

/// "It deals that much damage to target planeswalker that player controls."
fn damage_the_planeswalker(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, item.matched_damage_amount());
    Ok(())
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[
        Keyword::Vigilance,
        Keyword::Deathtouch,
        Keyword::Haste,
        Keyword::CantBeBlockedByPowerTwoOrLess,
    ]),
    unpreventable_damage: &[creatures_you_control_deal_unpreventable_combat_damage],
    abilities: &[triggered(when(EventMask::DAMAGE, triggers::deals_combat_damage_to_an_opponent), damage_the_planeswalker, "Whenever Questing Beast deals combat damage to an opponent, it deals that much damage to target planeswalker that player controls.").damage_amount()
        .targets(&[target(planeswalker_that_player_controls)])],
    ..oracle::QUESTING_BEAST
};
