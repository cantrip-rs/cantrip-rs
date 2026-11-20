use cantrip_api::*;
use cantrip_mechanics::combat::create_attacking_of_your_choice;
use cantrip_mechanics::{lands, mana, tokens, triggers};
use cantrip_oracle as oracle;

/// "Until end of turn, Den of the Bugbear becomes a 3/2 red Goblin creature with 'Whenever this creature attacks, create a
/// 1/1 red Goblin creature token that's tapped and attacking.' It's still a land." (CR 613.1d, 613.1f, 613.4b): the color and
/// creature type are the ones its own text has now (CR 612).
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ability = Some(AbilityRef::delayed(item.ability.card, 0));
    lands::becomes_creature_with_ability_until_end_of_turn(g, item, (3, 2), ColorSet::of(Color::Red), Subtype::Goblin, Keywords::NONE, ability);
    Ok(())
}

/// "Create a 1/1 red Goblin creature token that's tapped and attacking" (CR 508.4): its controller chooses what it attacks.
fn goblin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let goblin = g.card_id(&tokens::GOBLIN);
    create_attacking_of_your_choice(g, item.controller, item.object, goblin, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_if_two_or_more_other_lands),
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        ability(cost("{3}{R}"), becomes_creature, "{3}{R}: Until end of turn, Den of the Bugbear becomes a 3/2 red Goblin creature with \"Whenever this creature attacks, create a 1/1 red Goblin creature token that's tapped and attacking.\" It's still a land."),
    ],
    delayed: &[triggered(triggers::THIS_ATTACKS, goblin, "Whenever this creature attacks, create a 1/1 red Goblin creature token that's tapped and attacking.")],
    ..oracle::DEN_OF_THE_BUGBEAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
