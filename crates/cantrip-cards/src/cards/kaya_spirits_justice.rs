use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::library::still_exiled;
use cantrip_mechanics::select;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Whenever one or more creatures you control and/or creature cards in your graveyard are put into exile" (CR 603.2c, 603.10a):
/// a creature on the battlefield under this ability's controller, as it last was there, or a creature card in their graveyard
/// (a token or a copy is no card there, CR 108.2, 111.6), going to exile. The batch's member is the object in exile that it
/// became (CR 400.7), the card "you may choose ... from among them" is found among.
fn creature_you_control_or_in_your_graveyard_is_exiled(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, from, to, .. } if to.is(ZoneKind::Exile) => {
            let from_battlefield = from.is(ZoneKind::Battlefield) && q.obj(*old).controller == me.controller;
            let from_graveyard = *from == Zone::graveyard(me.controller) && !q.obj(*old).token && !q.obj(*old).card_copy;
            ((from_battlefield || from_graveyard) && q.chars(*old).is(CardType::Creature)).then(|| Fired::about(Target::Object(*new))).flatten()
        }
        _ => None,
    }
}

/// "You may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has
/// flying" (CR 707.2, 707.9a, 611.2): the batch's objects still in exile as it resolves (a card that left it is a new object, CR 400.7;
/// a token that was exiled has ceased to exist, CR 111.7), of which the creature cards are offered, a card being a creature card by its
/// printed types in exile, whatever it was on the battlefield. With the target token gone, the ability does nothing, not even ask.
fn token_becomes_a_copy_with_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(token) = targets::object(g, item, 0) else { return Ok(()) };
    let among = still_exiled(g, g.event_objects(item));
    let chosen = choose::select(g, item.controller, item.object, Purpose::CopyWhich, among, select::creature_cards(select::up_to(1)))?;
    if let Some(&card) = chosen.first() {
        let values = g.copiable_values(card).with_keyword(Keyword::Flying);
        g.become_copy(item.object, token, values, Duration::EndOfTurn);
    }
    Ok(())
}

/// "Surveil 2, then exile a card from a graveyard" (CR 701.25, 608.2c): the card is chosen by the controller from every graveyard,
/// the cards the surveil put there included; it is not a target.
fn surveil_then_exile_a_card_from_a_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)?;
    let cards: Vec<ObjectId> = g.state().apnap().flat_map(|p| g.state().zone(Zone::graveyard(p)).to_vec()).collect();
    let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, cards, select::cards(select::as_many_as_can(1)))?;
    g.exile_all(&chosen);
    Ok(())
}

/// "Create a 1/1 white and black Spirit creature token with flying."
fn create_a_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::WHITE_BLACK_FLYING_SPIRIT, 1)?;
    Ok(())
}

/// The slot for one other player of "for each other player, exile up to one target creature that player controls" (CR 601.2c, 115.1):
/// a creature that player controls, the core making a slot for each opponent and saying which it is (`Query::slot_opponent`).
fn creature_the_slots_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::controlled_by_slot_opponent(q, id) && filters::creature(q, asker, id))
}

const FOR_EACH_OTHER_PLAYER: TargetSpec = target(creature_the_slots_opponent_controls)
    .between(0, TargetSpec::EACH_OPPONENT)
    .labelled("Exile up to one target creature {player} controls");

/// "Exile target creature you control. For each other player, exile up to one target creature that player controls." Two
/// sentences, so your creature is exiled first and then the others, together (CR 608.2c, 608.2b: each only if still legal).
fn exile_yours_then_theirs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(mine) = targets::object(g, item, 0) {
        g.exile_all(&[mine]);
    }
    let theirs: Vec<ObjectId> = (1..item.target_count as usize).filter_map(|slot| targets::object(g, item, slot)).collect();
    g.exile_all(&theirs);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_you_control_or_in_your_graveyard_is_exiled),
            token_becomes_a_copy_with_flying,
            "Whenever one or more creatures you control and/or creature cards in your graveyard are put into exile, you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.",
        )
        .targets(&[targets::TOKEN_YOU_CONTROL])
        .once_per_batch(),
        ability(loyalty(2), surveil_then_exile_a_card_from_a_graveyard, "+2: Surveil 2, then exile a card from a graveyard."),
        ability(loyalty(1), create_a_spirit, "+1: Create a 1/1 white and black Spirit creature token with flying."),
        ability(loyalty(-2), exile_yours_then_theirs, "\u{2212}2: Exile target creature you control. For each other player, exile up to one target creature that player controls.")
            .targets(&[targets::CREATURE_YOU_CONTROL, FOR_EACH_OTHER_PLAYER]),
    ],
    ..oracle::KAYA_SPIRITS_JUSTICE
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLACK_FLYING_SPIRIT];
