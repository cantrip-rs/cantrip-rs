use cantrip_api::*;
use cantrip_mechanics::history::you_gained_life_this_turn;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature card with mana value X or less from your graveyard, where X is the amount of life you gained this turn"
/// (CR 202.3): the life gained is read from the history each time the target is judged, as it is chosen and again as the
/// ability resolves (CR 608.2b).
fn creature_card_with_mana_value_at_most_life_gained(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let gained = q.history().life_gained(asker.controller);
    targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Creature], gained)
}

/// "Return up to one target creature card ... from your graveyard to the battlefield."
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_pest_that_gains_life_on_attack,"When Moseo enters, create a 1/1 black and green Pest creature token with \"Whenever this token attacks, you gain 1 life.\""),
        triggered(YOUR_END_STEP, reanimate, "Infusion \u{2014} At the beginning of your end step, if you gained life this turn, return up to one target creature card with mana value X or less from your graveyard to the battlefield, where X is the amount of life you gained this turn.")
            .targets(&[up_to_one(creature_card_with_mana_value_at_most_life_gained)])
            .condition(you_gained_life_this_turn),
    ],
    ..oracle::MOSEO_VEINS_NEW_DEAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::PEST_THAT_GAINS_LIFE_ON_ATTACK];
