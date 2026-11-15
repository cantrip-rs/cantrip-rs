use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this enchantment enters, create a token that's a copy of target nontoken creature." The token is remembered by this
/// ability (CR 607.2c) for the linked abilities below: an ability a copy or grant effect gave the object later is not linked to
/// it (CR 607.5).
fn create_token_thats_copy_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    let made = g.create_token_copies(original, item.controller, 1)?;
    if let Some(&token) = made.first() {
        let by = g.linked(item);
        g.remember_chosen_object(by, token);
    }
    Ok(())
}

/// When this enchantment leaves, exile the token it made (CR 607.2c): the one remembered, if it is still on the battlefield.
fn exile_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let by = g.linked(item);
    let token: Vec<ObjectId> = g.state().chosen_object(by).filter(|&id| targets::permanent(g, id)).into_iter().collect();
    g.exile_all(&token);
    Ok(())
}

/// "When the token leaves the battlefield" (CR 603.6c, 603.10a): the token this ability's linked ability made, as it last existed.
fn when_token_leaves_battlefield(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, .. } if from.is(ZoneKind::Battlefield) && q.state().chosen_object(q.linked_to(asker)) == Some(*old) => Fired::PLAIN,
        _ => None,
    }
}

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}{U}."
fn sacrifice_unless_pay_uu(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::sacrifice_this_unless_pay(g, item, cost("{U}{U}"))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                create_token_thats_copy_target,
                "When this enchantment enters, create a token that's a copy of target nontoken creature.",
            )
            .targets(&[targets::NONTOKEN_CREATURE]),
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
            sacrifice_unless_pay_uu,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}{U}.",
        ),
    ],
    ..oracle::DANCE_OF_MANY
};
