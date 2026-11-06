use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchanted creature can't be blocked except by artifact creatures and/or white creatures." (CR 604.1).
fn enchanted_creature_cant_be_blocked_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(q.state().obj(asker.source).attached_to == Some(attacker)
        && !(((q.chars(blocker).is(CardType::Artifact)
            && q.chars(blocker).is(CardType::Creature))
            || (q.chars(blocker).colors.has(q.words(asker.source).color(Color::White))
                && q.chars(blocker).is(CardType::Creature)))))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[enchanted_creature_cant_be_blocked_rule],
    enchant: Some(targets::CREATURE),
    ..oracle::SEEKER
};
