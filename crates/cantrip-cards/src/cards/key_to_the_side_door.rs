use cantrip_api::*;
use cantrip_mechanics::{effects, same_name};
use cantrip_oracle as oracle;

/// "Discard a legendary card with the same name as a legendary permanent you control" (CR 701.9c, 201.2): a legendary card in the
/// activator's hand, and a legendary permanent that player controls (through the layers) with a name in common with it
/// (`same_name`: a copy has the name it copies).
fn legendary_card_named_like_a_legendary_permanent_you_control(q: &dyn Query, asker: &Asker, card: ObjectId) -> bool {
    q.chars(card).supertypes.legendary
        && q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).supertypes.legendary && same_name(q, card, id))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}, {T}"), effects::target_creature_cant_be_blocked_this_turn, "{2}, {T}: Target creature can't be blocked this turn.")
            .targets(&[cantrip_mechanics::targets::CREATURE]),
        ability(
            cost("{1}, {T}").and(discard(1, legendary_card_named_like_a_legendary_permanent_you_control)),
            |g, item| effects::draw(g, item, 2),
            "{1}, {T}, Discard a legendary card with the same name as a legendary permanent you control: Draw two cards.",
        ),
    ],
    ..oracle::KEY_TO_THE_SIDE_DOOR
};
