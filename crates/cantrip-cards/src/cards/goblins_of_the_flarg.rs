use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// A Dwarf controlled by the source's controller (CR 603.8, 612.1).
fn controls_dwarf(q: &dyn Query, asker: &Asker) -> bool {
    let dwarf = q.words(asker.source).subtype(Subtype::Dwarf);
    q.state().battlefield().iter().copied().any(|id| q.controller(id) == asker.controller && q.chars(id).has_subtype(dwarf))
}

pub static CARD: CardDef = CardDef {
    abilities: &[state_triggered(controls_dwarf, effects::sacrifice_self, "When you control a Dwarf, sacrifice this creature.")],
    ..oracle::GOBLINS_OF_THE_FLARG
};
