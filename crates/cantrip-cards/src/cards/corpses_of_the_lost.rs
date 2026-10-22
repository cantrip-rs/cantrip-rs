use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, plus_pt};
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 2/2 black Skeleton Pirate creature token.
pub static SKELETON_PIRATE: CardDef = CardDef {
    name: "Skeleton Pirate Token",
    token_name: Some(&[Subtype::Skeleton, Subtype::Pirate]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Skeleton, Subtype::Pirate]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

const ASK: Question = Question::new("ReturnToHand", "pay {cost} to return {subject} to its owner's hand?", "", "Pay {cost}", "Leave it");

/// "Skeletons you control" (CR 613.1f): creatures under the controller's control with the creature type Skeleton as
/// the text of this card reads it (CR 612).
fn your_skeletons(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.has_subtype(q.words(source).subtype(Subtype::Skeleton)) && q.state().obj(affected).controller == q.state().obj(source).controller
}

/// "Create a 2/2 black Skeleton Pirate creature token."
fn skeleton_pirate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SKELETON_PIRATE, 1)?;
    Ok(())
}

/// "If you descended this turn" (CR 700.11): a permanent card was put into the player's graveyard from anywhere this
/// turn, whether or not it is still there. A token is not a card (CR 111.7).
fn descended_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).to(ZoneKind::Graveyard).this_turn())
        .any(|r| {
            matches!(r.event, Event::ZoneChange { old, new, to, .. }
                if to == Zone::graveyard(asker.controller) && !q.obj(old).token && !q.obj(new).token && targets::is_permanent_card(&q.chars(new)))
        })
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
        .condition(descended_this_turn),
    ],
    statics: &[
        static_ability(Layer::PtModify, your_skeletons, plus_pt::<1, 0>, "Skeletons you control get +1/+0 and have haste."),
        static_ability(Layer::Ability, your_skeletons, have_haste, "Skeletons you control get +1/+0 and have haste."),
    ],
    ..oracle::CORPSES_OF_THE_LOST
};

pub static EXTRA: &[&CardDef] = &[&SKELETON_PIRATE];
