use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// The controller is remembered before countering changes the spell's zone (CR 400.7, 118.12).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let payer = g.controller(spell);
    let paid = g.may_pay(payer, item.object, spell, Cost::mana(generic(item.x as u32)), &PAY_QUESTION)?;
    if !paid {
        g.counter(spell);
    }
    g.end_sentence();
    if !paid {
        let lands: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| {
            let chars = g.chars(id);
            g.controller(id) == payer && chars.is(CardType::Land)
                && chars.abilities().any(|a| {
                    g.activated_ability(a).is_some_and(|d| !d.special_action && d.is_mana_ability())
                        || g.triggered_ability(a).is_some_and(|d| d.mana_ability)
                })
        }).collect();
        g.tap_together(&lands);
        g.empty_mana_pool(payer);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Countered",
    "pay {cost}?",
    "If you decline, {subject} is countered if able; tap all lands with mana abilities you control and lose all unspent mana",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], effect), ..oracle::POWER_SINK };
