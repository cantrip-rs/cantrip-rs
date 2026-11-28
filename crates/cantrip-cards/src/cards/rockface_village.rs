use cantrip_api::*;
use cantrip_mechanics::{haste_until_end_of_turn, pump, targets};
use cantrip_oracle as oracle;

/// "Target Lizard, Mouse, Otter, or Raccoon you control": a permanent you control with one of those creature types, the
/// words read through the land's own (CR 612).
fn critter_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Lizard, Subtype::Mouse, Subtype::Otter, Subtype::Raccoon])
        && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

/// "... gets +1/+0 and gains haste until end of turn": nothing if the target is no longer legal (CR 608.2b).
fn hurry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(critter) = targets::object(g, item, 0) {
        instruction(g, |g| {
            pump(g, item.object, critter, 1, 0);
            haste_until_end_of_turn(g, item.object, critter);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP, add(Mana::R).only_for(ManaRestriction::CREATURE_SPELLS), "{T}: Add {R}. Spend this mana only to cast a creature spell."),
        ability(
            cost("{R}, {T}"),
            hurry,
            "{R}, {T}: Target Lizard, Mouse, Otter, or Raccoon you control gets +1/+0 and gains haste until end of turn. Activate only as a sorcery.",
        )
        .targets(&[target(critter_you_control)])
        .sorcery_speed(),
    ],
    ..oracle::ROCKFACE_VILLAGE
};
