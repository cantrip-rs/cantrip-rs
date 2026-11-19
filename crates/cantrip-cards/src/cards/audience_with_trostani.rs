use cantrip_api::*;
use cantrip_mechanics::{effects, library, select, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/1 green Plant creature token, then draw cards equal to the number of differently named creature tokens you control."
/// Two sentences: the token is on the battlefield before the count (CR 608.2c), and so counts itself. The count is the largest set of
/// creature tokens you control with no name in common (CR 201.2a-b), so two Plants are one name; creatures that are not tokens and
/// tokens an opponent controls are not counted.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::PLANT, 1)?;
    let names: Vec<Vec<&'static str>> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.obj(id).controller == item.controller && g.obj(id).token && g.chars(id).is(CardType::Creature))
        .map(|id| library::object_names(g, id))
        .collect();
    effects::draw(g, item, select::largest_name_set(&names).len() as u32)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::AUDIENCE_WITH_TROSTANI };

pub static EXTRA: &[&CardDef] = &[&tokens::PLANT];
