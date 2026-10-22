use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// A 2/2 colorless Robot Villain artifact creature token (CR 111.4).
pub static ROBOT_VILLAIN: CardDef = CardDef {
    name: "Robot Villain Token",
    token_name: Some(&[Subtype::Robot, Subtype::Villain]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Robot, Subtype::Villain]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "Whenever one or more creature cards are put into your graveyard from anywhere" (CR 603.2c, 603.6a): asked of each card
/// that arrives in its controller's graveyard - a card, so not a token (CR 111.1) - and one trigger for those that arrive
/// together.
fn creature_cards_to_your_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    if *to == Zone::graveyard(me.controller) && !q.obj(*new).token && q.chars(*new).is(CardType::Creature) {
        Fired::PLAIN
    } else {
        None
    }
}

/// "You draw a card, lose 1 life, and put a plan counter on this enchantment." The counter goes only on the enchantment that
/// triggered, if it is still there (CR 400.7).
fn draw_lose_and_plan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw_a_card_and_lose_1_life(g, item)?;
    g.add_counters(item.object, CounterKind::Plan, 1);
    Ok(())
}

/// "When the third plan counter is put on this enchantment, sacrifice it and create three 2/2 colorless Robot Villain
/// artifact creature tokens." The tokens are made whether or not it is still there to be sacrificed.
fn robots(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(&[item.object]);
    tokens::create(g, item.controller, &ROBOT_VILLAIN, 3)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_cards_to_your_graveyard),
            draw_lose_and_plan,
            "Whenever one or more creature cards are put into your graveyard from anywhere, you draw a card, lose 1 life, and put a plan counter on this enchantment.",
        )
        .once_per_batch(),
        triggered(
            when_asked(EventMask::COUNTERS, triggers::nth_counter_put_on_this::<{ CounterKind::Plan as u8 }, 3>),
            robots,
            "When the third plan counter is put on this enchantment, sacrifice it and create three 2/2 colorless Robot Villain artifact creature tokens.",
        ),
    ],
    ..oracle::ROBOT_DOMINATION
};

pub static EXTRA: &[&CardDef] = &[&ROBOT_VILLAIN];
