use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Triumphant Chomp deals damage to target creature equal to 2 or the greatest power among Dinosaurs you control, whichever is
/// greater" (CR 208.1): the greatest power among the Dinosaurs the caster controls as this resolves (CR 608.2h), through the
/// Dinosaur word of this spell's own text (CR 612.2); with none, or only a power below 2 (a negative one counts as 0, CR 107.1b), it deals 2.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let dinosaur = g.words(item.object).subtype(Subtype::Dinosaur);
    let greatest = g
        .permanents_of(item.controller)
        .into_iter()
        .map(|id| g.chars(id))
        .filter(|c| c.has_subtype(dinosaur))
        .filter_map(|c| c.power)
        .fold(2, i32::max);
    damage_target(g, item, 0, greatest as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::TRIUMPHANT_CHOMP };
