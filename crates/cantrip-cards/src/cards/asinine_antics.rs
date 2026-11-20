use cantrip_api::*;
use cantrip_mechanics::{flash_cost, tokens};
use cantrip_oracle as oracle;

/// "For each creature your opponents control, create a Cursed Role token attached to that creature" (CR 111.10q, 303.7, 101.4):
/// the creatures each opponent controls as it resolves, one Role for each, all created as one instruction. The Roles are under
/// your control, and a Role of yours already on a creature goes to the graveyard as the new one enters (CR 303.7a, 704.5z); an
/// opponent's own Role there stays. The spell does not target.
fn cursed_role_on_each_creature_your_opponents_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g
        .state()
        .opponents(item.controller)
        .flat_map(|opponent| g.permanents_of(opponent))
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .collect();
    tokens::create_roles(g, &tokens::CURSED_ROLE, item.controller, &creatures)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(flash_cost::OPTIONAL_COST),
    labels: flash_cost::LABELS,
    spell: spell(&[], cursed_role_on_each_creature_your_opponents_control),
    ..oracle::ASININE_ANTICS
};

pub static EXTRA: &[&CardDef] = &[&tokens::CURSED_ROLE];
