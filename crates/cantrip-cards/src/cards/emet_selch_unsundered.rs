use cantrip_api::*;
use cantrip_mechanics::effects::{ask_may, loot};
use cantrip_mechanics::triggers::{THIS_ENTERS_OR_ATTACKS, YOUR_UPKEEP};
use cantrip_oracle as oracle;

const TRANSFORM: Question = Question::new("TransformEmetSelch", "transform Emet-Selch?", "It becomes Hades, Sorcerer of Eld", "Transform", "Don't transform");

/// The upkeep ability's intervening "if" (CR 603.4): fourteen or more cards in its controller's graveyard, checked as the
/// upkeep begins and again as the ability resolves.
fn fourteen_cards_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    q.state().zone(Zone::graveyard(asker.controller)).len() >= 14
}

/// "You may transform Emet-Selch." The ability's source, which does nothing if it is no longer that object (CR 400.7) or has
/// transformed since the ability went on the stack (CR 701.27f).
fn may_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may(g, item.controller, item.object, Purpose::Ask(&TRANSFORM))? {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

/// "During your turn, you may play cards from your graveyard" (CR 604, 305.1, 601.2): a land is played for nothing (the land
/// drop is the engine's), any other card is cast for its mana cost, and a card with no mana cost can't be (CR 118.6). A
/// permission (`GrantedWay::permission`), so an alternative cost may be paid instead. Only its controller's own cards, in their
/// own graveyard, on their own turn.
fn from_your_graveyard(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller || q.state().turn.active != player || q.obj(card).owner != player {
        return None;
    }
    let c = q.chars(card);
    if c.is(CardType::Land) {
        return Some(Cost::NONE);
    }
    let def = q.card(c.card);
    def.has_cost.then(|| Cost::mana(def.cost))
}

pub static BACK: CardDef = CardDef {
    granted_ways: &[GrantedWay {
        text: "Echo of the Lost \u{2014} During your turn, you may play cards from your graveyard.",
        cost: from_your_graveyard,
        lands: true,
        permission: true,
        ..GrantedWay::CAST
    }],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        cantrip_mechanics::replacements::card_or_token_to_your_graveyard,
        cantrip_mechanics::replacements::exile_instead_apply,
        "exiles a card or token your graveyard would receive",
    )],
    ..oracle::HADES_SORCERER_OF_ELD
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS_OR_ATTACKS, loot, "Whenever Emet-Selch enters or attacks, draw a card, then discard a card."),
        triggered(
            YOUR_UPKEEP,
            may_transform,
            "At the beginning of your upkeep, if there are fourteen or more cards in your graveyard, you may transform Emet-Selch.",
        )
        .condition(fourteen_cards_in_your_graveyard),
    ],
    back: Some(&BACK),
    ..oracle::EMET_SELCH_UNSUNDERED
};
