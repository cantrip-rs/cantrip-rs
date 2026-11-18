use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, plus_pt};
use cantrip_mechanics::{history, lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// The 2/2 black Skeleton Pirate creature token.
pub static SKELETON_PIRATE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Skeleton Pirate)
};

const ASK: Question = Question::new("ReturnToHand", "pay {cost} to return {subject} to its owner's hand?", "", "Pay {cost}", "Leave it");

/// "Create a 2/2 black Skeleton Pirate creature token."
fn skeleton_pirate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SKELETON_PIRATE, 1)?;
    Ok(())
}

/// "You may pay 1 life. If you do, return this enchantment to its owner's hand." The life is paid as the ability
/// resolves, even if the enchantment has left, by a player with the life to pay (CR 119.4); it is returned only if it is
/// still on the battlefield.
fn pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, life(1), &ASK)? {
        g.return_to_hand(&[item.object]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, skeleton_pirate, "When this enchantment enters, create a 2/2 black Skeleton Pirate creature token."),
        triggered(
            triggers::YOUR_END_STEP,
            pay_to_return,
            "At the beginning of your end step, if you descended this turn, you may pay 1 life. If you do, return this enchantment to its owner's hand.",
        )
        .condition(history::descended_this_turn),
    ],
    statics: &[
        static_ability(Layer::PtModify, statics::your_of_subtype::<{ lands::sub(Subtype::Skeleton) }>, plus_pt::<1, 0>, "Skeletons you control get +1/+0 and have haste."),
        static_ability(Layer::Ability, statics::your_of_subtype::<{ lands::sub(Subtype::Skeleton) }>, have_haste, "Skeletons you control get +1/+0 and have haste."),
    ],
    ..oracle::CORPSES_OF_THE_LOST
};

pub static EXTRA: &[&CardDef] = &[&SKELETON_PIRATE];
