use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets};
use cantrip_oracle as oracle;

/// "Target nonartifact, nonblack creature" (CR 115.1).
fn nonartifact_nonblack_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Creature) && !c.is(CardType::Artifact)
            && !c.colors.has(q.words(asker.source).color(Color::Black))
    })
}

/// "Destroy target nonartifact, nonblack creature. It can't be regenerated" (CR 701.8a, 701.19c).
fn destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target_no_regen(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(nonartifact_nonblack_creature)], destroy),
    ..oracle::TERROR
};
