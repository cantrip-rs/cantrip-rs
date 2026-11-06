use cantrip_api::*;
use cantrip_mechanics::{counters, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {G}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_enchantment_unless_pay_g(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{G}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "Whenever a player puts a Swamp onto the battlefield" (CR 603.2).
fn when_player_puts_swamp_onto_battlefield(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, to, put_by: Some(put_by), .. } = event else { return None };
    let id = *new;
    let player = *put_by;
    let subject = Target::Object(id);
    let data = [new.0, u32::from(player.0)];
    if to.is(ZoneKind::Battlefield)
        && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Swamp))
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever a player puts a Swamp onto the battlefield, this enchantment deals 3 damage to that player unless the player puts a -1/-1 counter on a creature they control." (CR 608.2, 120.3, 118.1).
fn enchantment_deals_3_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let player = PlayerId(item.data[1] as u8);
    let creatures = g.permanents_of(player).into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .collect();
    if counters::may_pay_on_one(g, player, item.object, CounterKind::MinusOne, 1, creatures, &COUNTER_QUESTION)?.is_none() {
        g.damage(item.object, Target::Player(player), 3);
    }
    Ok(())
}

static COUNTER_QUESTION: Question = Question::new(
    "PutCounter", "put a -1/-1 counter on a creature you control?",
    "If you don't, this deals 3 damage to you.", "Put a counter", "Take damage",
).no_default();

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
            triggers::YOUR_UPKEEP,
            sacrifice_enchantment_unless_pay_g,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {G}.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_player_puts_swamp_onto_battlefield),
            enchantment_deals_3_damage_player,
            "Whenever a player puts a Swamp onto the battlefield, this enchantment deals 3 damage to that player unless the player puts a -1/-1 counter on a creature they control.",
        ),
    ],
    ..oracle::THELONS_CHANT
};
