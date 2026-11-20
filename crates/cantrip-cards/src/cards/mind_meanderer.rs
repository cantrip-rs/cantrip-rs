use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_mechanics::statics::{have_vigilance, itself};
use cantrip_oracle as oracle;

/// "As long as you control a Jace planeswalker": any Jace planeswalker permanent its controller controls, token or not
/// (CR 109.5, 113.6), read as the layer is applied, so it is lost and regained with the planeswalker.
fn you_control_a_jace(q: &dyn LayerQuery, source: ObjectId) -> bool {
    let you = q.state().obj(source).controller;
    q.state().battlefield().iter().copied().any(|id| {
        let c = q.chars(id);
        c.is(CardType::Planeswalker) && c.has_subtype(Subtype::Jace) && q.state().obj(id).controller == you
    })
}

fn vigilance_while_jace(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    itself(q, source, affected, so_far) && you_control_a_jace(q, source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        cantrip_mechanics::triggers::THIS_ENTERS,
        fight::this_fights_first_target,
        "When this creature enters, it fights up to one target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.between(0, 1)])],
    statics: &[static_ability(
        Layer::Ability,
        vigilance_while_jace,
        have_vigilance,
        "This creature has vigilance as long as you control a Jace planeswalker.",
    )],
    ..oracle::MIND_MEANDERER
};
