use cantrip_api::*;
use cantrip_mechanics::hand;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const KEEP: Question = Question::new("Sacrificed", "pay {cost}, or {subject} is sacrificed?", "", "Pay {cost}", "Sacrifice it");

/// You may put a creature card from your hand onto the battlefield. If you do, sacrifice it unless you pay its mana
/// cost reduced by {2}. The card is chosen as the spell resolves (CR 608.2d); one that cannot be paid for (a card with
/// no mana cost) is sacrificed outright.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let Some(entered) = hand::may_put_onto_battlefield(g, player, item.object, |c| c.is(CardType::Creature))? else { return Ok(()) };
    if !targets::permanent(g, entered) {
        return Ok(());
    }
    let chars = g.chars(entered);
    let mut cost = g.mana_cost(entered);
    cost.generic = cost.generic.saturating_sub(2);
    cost.x = 0; // X in the mana cost is zero (CR 107.3g).
    if chars.no_cost || !g.card(chars.card).has_cost || !g.may_pay(player, item.object, entered, of_mana(cost), &KEEP)? {
        g.sacrifice(player, &[entered]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FLASH };
