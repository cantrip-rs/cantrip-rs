use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets};
use cantrip_oracle as oracle;

/// Target a Wall, reading its creature type through this spell's text (CR 115.1, 612.2).
fn wall(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Wall])
}

/// Destroy the Wall without regeneration, then damage its controller using its current or last known information (CR 701.19c, 608.2h).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(wall) = destroy_target_no_regen(g, item, 0) {
        g.end_sentence();
        g.damage_together(&[(item.object, Target::Player(g.controller(wall)), g.mana_value(wall))]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(wall)], effect),
    ..oracle::WORD_OF_BLASTING
};
