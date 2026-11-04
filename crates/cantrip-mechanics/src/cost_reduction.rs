//! "This spell costs {1} less to cast for each ..." where the card is not affinity (CR 702.41a is the keyword, and only the cards
//! that print the word use `affinity`): a cost reduction (CR 601.2f, 118.7) that counts a pool of the caster's. Also the
//! permanents' "[spells] you cast cost {1} less to cast".

use cantrip_api::*;

/// "This spell costs {1} less to cast for each instant and sorcery card in your graveyard" (CR 601.2f, 118.7a): counted as
/// the cost is determined, the cards as they are in the graveyard. Eddymurk Crab and The Dawning Archaic print it.
pub fn for_each_instant_and_sorcery_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let cards = q.state().zone(Zone::graveyard(player)).iter().filter(|&&id| q.chars(id).is(CardType::Instant) || q.chars(id).is(CardType::Sorcery)).count();
    CostDelta::less(cards as u32)
}

/// "This spell costs {N} less to cast if you control a [subtype]" (CR 601.2f, 118.7a): judged once, as the total cost is
/// determined, for the cost of casting it only (the core asks a card's own change only as it is cast), and counting only the
/// caster's permanents. `TYPE` is `lands::sub(Subtype::Kithkin)` and so on; the type is a word of the spell's rules text, read
/// through its words (CR 612.2), and a permanent has it when the layers made it that type. Mistmeadow Council (Kithkin), Visions of Villainy (Villain), Saved by the
/// Shell (Turtle), Truck Toss (Vehicle, {2}) and Geistlight Snare (Spirit) print it.
pub fn less_if_you_control_subtype<const N: u8, const TYPE: u16>(q: &dyn Query, source: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let wanted = crate::lands::word(q, source, TYPE);
    if q.permanents_of(player).into_iter().any(|id| q.chars(id).has_subtype(wanted)) { CostDelta::less(N as u32) } else { CostDelta::NONE }
}

/// "[Color] spells you cast cost {1} less to cast" (CR 601.2f, 118.7c): asked of the spell's own characteristics as it is cast;
/// only its controller's spells, and a generic reduction takes only the generic part. `COLOR` is `Color::Red as u8` and so on,
/// a word of the source's text (CR 612). The Fire Crystal (red) and The Wind Crystal (white) print it.
pub fn spells_of_color_you_cast_cost_one_less<const COLOR: u8>(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) {
        return CostDelta::NONE;
    }
    if q.chars(spell).colors.has(q.words(source).color(Color::ALL[COLOR as usize])) { CostDelta::less(1) } else { CostDelta::NONE }
}
