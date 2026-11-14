use cantrip_api::*;
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} for destroyed lands?", "Unpaid lands cause damage", "Pay {cost}", "Decline");

/// The actual destroyed permanents supply their last controllers (CR 608.2h, 701.8a).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let plains = g.words(item.object).subtype(Subtype::Plains);
    let lands: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|&id| g.chars(id).has_subtype(plains)).collect();
    let destroyed = g.destroy_all(&lands);
    let payments: Vec<_> = g.state().apnap().map(|player| {
        let count = destroyed.permanents().iter().filter(|&&id| g.controller(id) == player).count();
        (player, count_u32(count))
    }).filter(|&(_, count)| count != 0).collect();
    g.end_sentence();
    let mut damage = Vec::new();
    for (player, count) in payments {
        let paid = g.may_pay_times(player, item.object, item.object, mana("{2}"), count, &PAY)?;
        if paid < count { damage.push((item.object, Target::Player(player), count - paid)); }
    }
    g.damage_together(&damage);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], resolve), ..oracle::STENCH_OF_EVIL };
