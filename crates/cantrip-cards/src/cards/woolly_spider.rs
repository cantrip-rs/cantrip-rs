use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever this creature blocks a creature with flying" (CR 603.2).
fn when_creature_blocks_creature_flying(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *blocker;
    let other = *attacker;
    let subject = Target::Object(id);
    if asker.source == id
        && (q.chars(other).is(CardType::Creature)
            && q.chars(other).keywords.has(Keyword::Flying))
    {
        return Fired::about(subject);
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_creature_flying),
            effects::this_gets::<0, 2>,
            "Whenever this creature blocks a creature with flying, this creature gets +0/+2 until end of turn.",
        ),
    ],
    ..oracle::WOOLLY_SPIDER
};
