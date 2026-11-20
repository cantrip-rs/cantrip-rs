use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "One or two target creatures each gain haste until end of turn. For each of those creatures, create a Monster Role token
/// attached to it." The creatures still legal as it resolves (CR 608.2b) gain haste as one instruction; the Roles follow for
/// those same creatures, created together as one event (CR 603.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    effects::targets_gain(g, item, Keyword::Haste)?;
    g.end_sentence();
    tokens::create_roles(g, &tokens::MONSTER_ROLE, item.controller, &creatures)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.between(1, 2)], effect), ..oracle::BECOME_BRUTES };

pub static EXTRA: &[&CardDef] = &[&tokens::MONSTER_ROLE];
