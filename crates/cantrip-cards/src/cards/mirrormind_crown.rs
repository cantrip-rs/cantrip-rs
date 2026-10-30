use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_oracle as oracle;

const COPY_INSTEAD: Question = Question::new(
    "CreateCopiesOfEquippedCreature",
    "create copies of the equipped creature instead?",
    "The tokens you would create are that many tokens that are copies of the equipped creature",
    "Create copies",
    "Create the tokens",
);

/// "As long as this Equipment is attached to a creature, the first time you would create one or more tokens each turn"
/// (CR 111.1, 614.1a): tokens that are yours, this turn's first creation. What was created earlier this turn is read from the
/// history (a token created as a copy of a permanent spell is not created, CR 111.13), and a creation the player declined to
/// replace still used the turn's first time up.
fn first_tokens_you_would_create_this_turn(q: &dyn Query, me: &Asker, p: &Proposal) -> bool {
    if !matches!(p, Proposal::TokenCreation(t) if t.controller == me.controller) || q.attached_to(me.source).is_none() {
        return false;
    }
    !q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).to(ZoneKind::Battlefield).reason(ZoneReason::Created).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { new, .. } if q.obj(new).owner == me.controller))
}

/// "... you may instead create that many tokens that are copies of equipped creature": the same count, each a copy of the
/// creature's copiable values (CR 707.2) - a token's own characteristics if it is one, what it copies if it is a copy.
fn copies_of_equipped_creature(q: &dyn Query, me: &Asker, p: &mut Proposal) {
    let (Proposal::TokenCreation(t), Some(creature)) = (p, q.attached_to(me.source)) else { return };
    let values = q.copiable_values(creature);
    t.token = values.card;
    t.copy = Some(values);
}

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    replacements: &[replacement(
        ReplacementKind::TokenCreation,
        first_tokens_you_would_create_this_turn,
        copies_of_equipped_creature,
        "creates that many tokens that are copies of the equipped creature instead",
    )
    .optional(&COPY_INSTEAD)],
    ..oracle::MIRRORMIND_CROWN
};
