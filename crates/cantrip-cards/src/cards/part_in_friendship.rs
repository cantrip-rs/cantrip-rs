use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "Whenever a nontoken creature you control dies" (CR 700.4, 603.10a): a dies ability, looked back on, so it also fires for a
/// creature that died in the same event as this permanent; the creature as it last was on the battlefield, under the controller it
/// last had. A token that dies is no nontoken creature.
fn nontoken_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Creature)
                && q.obj(*old).controller == me.controller
                && !q.obj(*old).token =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Reveal cards from the top of your library until you reveal a creature card. If its mana value is less than or equal to the
/// number of lands you control, put it onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your
/// library in a random order." (CR 701.20, 401.4, 608.2c): the lands are counted after the reveal, as the layers give them now, and
/// the card's mana value is the one it has in the library (an {X} in it is 0, CR 202.3e, the ruling). A library holding no creature
/// card is revealed whole and put back in a random order.
fn reveal_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let (found, rest) = library::reveal_until(g, you, &|q, card| q.chars(card).is(CardType::Creature));
    if let Some(card) = found {
        let lands = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Land)).count() as u32;
        if g.mana_value(card) <= lands {
            g.put_onto_battlefield(card, you, false)?;
        } else {
            g.return_to_hand_revealed(&[card]);
        }
    }
    library::rest_to_bottom_in_random_order(g, you, &rest);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, nontoken_creature_you_control_dies),
        reveal_a_creature,
        "Whenever a nontoken creature you control dies, reveal cards from the top of your library until you reveal a creature card. If its mana value is less than or equal to the number of lands you control, put it onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your library in a random order. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::PART_IN_FRIENDSHIP
};
