use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, exile_target_until_leaves_effect, targets};
use cantrip_oracle as oracle;

/// "Other target non-Fox creature" (CR 115.1, 205.3): a creature on the battlefield other than the Bodyguard that does not have
/// the Fox creature type, judged as the creature is now (a changeling is a Fox).
fn other_non_fox_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id)
        if targets::permanent(q, id)
            && id != asker.source
            && q.chars(id).is(CardType::Creature)
            && !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Fox)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            exile_target_until_leaves_effect,
            "When this creature enters, exile up to one other target non-Fox creature until this creature leaves the battlefield.",
        )
        .targets(&[up_to_one(other_non_fox_creature)]),
        ability(cost("{1}{W}").and(SACRIFICE_THIS), |g, item| effects::gain_life(g, item, 2), "{1}{W}, Sacrifice this creature: You gain 2 life."),
    ],
    ..oracle::WEREFOX_BODYGUARD
};
