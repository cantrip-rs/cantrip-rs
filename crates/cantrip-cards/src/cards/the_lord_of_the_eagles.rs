use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the total power of creatures you control with flying." (CR 601.2f, 118.7a,
/// 208.1): the sum of the power of each creature the caster controls that has flying as the cost is determined - one with
/// negative power counts as negative (CR 107.1b treats only a negative total as 0) - taking off generic mana only. Only the
/// cost of casting it: "this spell" is a spell. The reduction is capped at {255}, a cost's own width, far beyond the {7} of
/// generic mana the card has.
fn less_per_flying_power(q: &dyn Query, _: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let spell = matches!(costed, Costed::Spell(_));
    let total: i32 = q
        .permanents_of(player)
        .into_iter()
        .filter(|&id| spell && q.chars(id).is(CardType::Creature) && q.chars(id).keywords.has(Keyword::Flying))
        .map(|id| i32::from(q.chars(id).power.unwrap_or(0)))
        .sum();
    // A negative total (power can be below zero) reduces nothing: a cost never goes up from a reduction (CR 107.1b, 601.2f).
    // The clamp at 255 loses nothing: the generic mana it reduces is itself a byte (docs/rules-exceptions.md RX-5).
    CostDelta::less(total.clamp(0, 255) as u8)
}

pub static CARD: CardDef = CardDef { own_cost_changes: &[less_per_flying_power], ..oracle::THE_LORD_OF_THE_EAGLES };
