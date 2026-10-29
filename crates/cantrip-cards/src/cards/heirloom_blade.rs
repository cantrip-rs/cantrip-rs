use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("RevealUntilSharesCreatureType", "reveal until you reveal a creature card that shares a creature type?", "That card goes into your hand; the rest go on the bottom of your library in a random order", "Reveal", "Don't reveal");

/// "Whenever equipped creature dies" (CR 603.6c, 603.10a): the creature the Blade was attached to, judged on the board as
/// it was before. The ability triggers even when the Blade leaves at the same time. The dead creature is carried as the
/// ability's subject, and its creature types are read from it as it last existed on the battlefield (CR 608.2h).
fn equipped_creature_dies(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) && q.obj(source).attached_to == Some(*old) =>
        {
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

/// You may reveal cards from the top of your library until you reveal a creature card that shares a creature type with
/// it. Put that card into your hand and the rest on the bottom of your library in a random order. A dead creature with no
/// creature type shares one with nothing, so the whole library is revealed and randomized.
fn reveal_until_shared(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Target::Object(dead) = item.subject else { return Ok(()) };
    let types = g.chars(dead).subtypes;
    if !cantrip_mechanics::effects::ask_may(g, you, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let (found, revealed) = cantrip_mechanics::library::reveal_until(g, you, &|q, card| {
        let c = q.chars(card);
        c.is(CardType::Creature) && cantrip_mechanics::statics::shares_creature_type(types, c.subtypes)
    });
    if found.is_none() && revealed.is_empty() {
        return Ok(());
    }
    if let Some(card) = found {
        g.return_to_hand_revealed(&[card]);
    }
    g.bottom_in_random_order(you, &revealed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        equip(mana("{1}"), "Equip {1}"),
        triggered(when(EventMask::ZONE_CHANGE, equipped_creature_dies), reveal_until_shared, "Whenever equipped creature dies, you may reveal cards from the top of your library until you reveal a creature card that shares a creature type with it. Put that card into your hand and the rest on the bottom of your library in a random order."),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<3, 1>, "Equipped creature gets +3/+1.")],
    ..oracle::HEIRLOOM_BLADE
};
