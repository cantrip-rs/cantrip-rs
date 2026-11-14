use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever another Klingon or Warrior you control enters" (CR 603.6a): a permanent other than this one, under this
/// creature's controller's control as it entered, that is a Klingon or a Warrior (a token too), the types read through the
/// words of this card's text (CR 612); once for that permanent, though it is both.
fn another_klingon_or_warrior_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Klingon) }>(q, me, event)
        .or_else(|| triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Warrior) }>(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_klingon_or_warrior_enters),
        |g, item| {
            effects::damage_each_opponent(g, item, 1);
            Ok(())
        },
        "Whenever another Klingon or Warrior you control enters, this creature deals 1 damage to each opponent.",
    )],
    cost_changes: &[cost_reduction::either_subtype_spells_you_cast::<1, { lands::sub(Subtype::Klingon) }, { lands::sub(Subtype::Warrior) }>],
    ..oracle::GAGH_SLINGER
};
