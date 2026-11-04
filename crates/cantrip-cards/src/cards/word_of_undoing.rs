use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Return the target and its qualifying Auras together (CR 608.2b, 608.2c).
fn return_creature_and_auras(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let white = g.words(item.object).color(Color::White);
    let aura = g.words(item.object).subtype(Subtype::Aura);
    let mut objects = vec![creature];
    objects.extend(g.state().battlefield().iter().copied().filter(|&id| {
        let chars = g.chars(id);
        chars.colors.has(white) && chars.has_subtype(aura)
            && g.obj(id).owner == item.controller && g.obj(id).attached_to == Some(creature)
    }));
    g.return_to_hand(&objects);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], return_creature_and_auras),
    ..oracle::WORD_OF_UNDOING
};
