use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever another Frog, Rabbit, Raccoon, or Squirrel you control enters" (CR 603.6a): a permanent other than this one, under this
/// creature's controller's control as it entered, that is any of the four (a token too), the types read through the words of this
/// card's text (CR 612); once for that permanent, though it is several (a changeling, say). Creatures that enter together with
/// this one each trigger it separately (the 2024-07-26 ruling): this one sees them enter as it does (CR 603.6a).
fn another_frog_rabbit_raccoon_or_squirrel_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Frog) }>(q, me, event)
        .or_else(|| triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Rabbit) }>(q, me, event))
        .or_else(|| triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Raccoon) }>(q, me, event))
        .or_else(|| triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Squirrel) }>(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_frog_rabbit_raccoon_or_squirrel_enters),
        effects::plus_one_counter_on_this,
        "Whenever another Frog, Rabbit, Raccoon, or Squirrel you control enters, put a +1/+1 counter on this creature.",
    )],
    ..oracle::VALLEY_MIGHTCALLER
};
