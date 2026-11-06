use cantrip_api::*;
use cantrip_mechanics::permanents_with_subtype_you_control;
use cantrip_oracle as oracle;

/// Each opponent takes damage equal to their own Islands as the spell resolves, simultaneously (CR 608.2h, 120.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let island = g.words(item.object).subtype(Subtype::Island);
    let hits: Vec<_> = g.state().apnap()
        .filter(|&p| g.state().opponents(item.controller).any(|opponent| opponent == p))
        .map(|p| {
            let count = permanents_with_subtype_you_control(g, p, island);
            (item.object, Target::Player(p), count as u32)
        })
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::TYPHOON };
