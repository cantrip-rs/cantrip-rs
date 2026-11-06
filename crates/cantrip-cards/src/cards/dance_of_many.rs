use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this enchantment enters, create a token that's a copy of target nontoken creature." (CR 115.1).
fn create_token_thats_copy_target_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .state().obj(id).token) && q.chars(id).is(CardType::Creature)))
    )
}

/// "When this enchantment enters, create a token that's a copy of target nontoken creature." (CR 608.2, 111.1).
fn create_token_thats_copy_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    g.create_token_copies(original, item.controller, 1)?;
    Ok(())
}

/// Whether this token was made by this Dance's first ability (CR 607.2c).
fn linked_token(q: &dyn Query, by: Linked, token: ObjectId) -> bool {
    q.obj(token).token && q.history().all().iter().any(|r| {
        matches!(r.event, Event::Entered { permanent, .. } if permanent == token)
            && matches!(r.cause, Cause::Ability { source, ability, acquired } if source == by.object && q.linked_card(ability.card) == by.text && acquired == by.acquired && ability.card == q.card_id(&CARD) && ability.index == 0)
    })
}

/// When this enchantment leaves, exile the tokens made by its linked ability (CR 607.2c).
fn exile_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tokens: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|&id| linked_token(g, g.linked(item), id)).collect();
    g.exile_all(&tokens);
    Ok(())
}

/// When a linked token leaves the battlefield (CR 603.6c, 603.10a).
fn when_token_leaves_battlefield(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, from, .. } = event else { return None };
    if from.is(ZoneKind::Battlefield) && linked_token(q, q.linked_to(asker), *old) {
        Fired::about(Target::Object(*old))
    } else { None }
}

fn sacrifice_enchantment_unless_pay_u(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{U}{U}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                create_token_thats_copy_target,
                "When this enchantment enters, create a token that's a copy of target nontoken creature.",
            )
            .targets(&[target(create_token_thats_copy_target_target_0)]),
        triggered(
            triggers::THIS_LEAVES,
            exile_token,
            "When this enchantment leaves the battlefield, exile the token.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_token_leaves_battlefield),
            effects::sacrifice_self,
            "When the token leaves the battlefield, sacrifice this enchantment.",
        ),
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_enchantment_unless_pay_u,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}{U}.",
        ),
    ],
    ..oracle::DANCE_OF_MANY
};
