use cantrip_api::*;
use cantrip_mechanics::{fight, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "If damage would be dealt to Wolverine, instead that damage is dealt, but all other damage already dealt to him is
/// healed."
fn damage_to_him(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Object(asker.source) && d.amount > 0)
}

fn heal_the_rest(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.heal_marked = true;
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, fight::this_fights_first_target, "When Wolverine enters, he fights up to one other target creature.")
        .targets(&[up_to_one(targets::other_creature_filter)])],
    replacements: &[replacement(ReplacementKind::Damage, damage_to_him, heal_the_rest, "heal damage already dealt to Wolverine")],
    ..oracle::WOLVERINE_FIERCE_FIGHTER
};
