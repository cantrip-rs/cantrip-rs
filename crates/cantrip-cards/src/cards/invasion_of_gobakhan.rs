use cantrip_api::*;
use cantrip_mechanics::{hand, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its
/// owner may play it. A spell cast this way costs {2} more to cast." Looking at someone else's hand is worth doing even
/// with nothing eligible, so the choice is asked of any hand that is not empty.
fn take_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(opponent) = item.target(0) else { return Ok(()) };
    let Some(card) = hand::look_and_may_choose(g, item, opponent, Purpose::Exile, "nonland", |q, c| !q.chars(c).is(CardType::Land))? else { return Ok(()) };
    hand::exile_owner_may_play_for_two_more(g, card);
    Ok(())
}

// --- Lightshield Array --------------------------------------------------------------------------------------------

/// "Put a +1/+1 counter on each creature that attacked this turn": whoever controls it now, each one still the object
/// that attacked (CR 400.7) and still a creature on the battlefield.
fn counters_on_attackers(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.attacked_this_turn(id) && g.chars(id).is(CardType::Creature))
        .collect();
    let puts: Vec<(ObjectId, CounterKind, i64)> = attackers.into_iter().map(|id| (id, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

/// "Creatures you control gain hexproof and indestructible until end of turn."
fn protect_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::creatures_you_control_gain(g, item, &[Keyword::Hexproof, Keyword::Indestructible]);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::YOUR_END_STEP, counters_on_attackers, "At the beginning of your end step, put a +1/+1 counter on each creature that attacked this turn."),
        ability(SACRIFICE_THIS, protect_creatures, "Sacrifice this enchantment: Creatures you control gain hexproof and indestructible until end of turn."),
    ],
    ..oracle::LIGHTSHIELD_ARRAY
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, take_a_card, "When this Siege enters, look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.")
        .targets(&[targets::OPPONENT])],
    back: Some(&BACK),
    ..oracle::INVASION_OF_GOBAKHAN
};
