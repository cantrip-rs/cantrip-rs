use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "The amount of {B} spent on X" (CR 107.3, 601.2h): the black mana that paid X's part of the cost as `spell` was cast,
/// which the payer chose as they paid (`Event::SpentOnX`); mana spent as though it were black is not black (CR 609.4b). None
/// for a copy, which was not cast (CR 707.10), or for a spell cast without paying its mana cost. A spell is cast and resolves
/// in one turn, so this turn's records hold it.
fn black_spent_on_x(q: &dyn Query, spell: ObjectId) -> i64 {
    q.history()
        .this_turn()
        .iter()
        .filter_map(|r| match r.event {
            Event::SpentOnX { on, cast: true, mana: Mana::B, n, .. } if on == spell => Some(i64::from(n)),
            _ => None,
        })
        .sum()
}

/// "Soul Burn deals X damage to any target. You gain life equal to the damage dealt, but not more than the amount of {B}
/// spent on X, the player's life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt,
/// or the creature's toughness." The damage and every cap but the {B} are Drain Life's
/// (`effects::deal_damage_for_drained_life`); the ruling of 2004-10-04: "1 life for each black mana used, but not more life
/// than the amount of unprevented damage that is dealt". The {B} was spent as the spell was cast, before anything resolves.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let black = black_spent_on_x(g, item.object);
    let life = effects::deal_damage_for_drained_life(g, item.object, item.target(0), u32::from(item.x)).min(black);
    g.gain_life(item.controller, life as i32);
    Ok(())
}

/// "Spend only black and/or red mana on X." (CR 107.3, 601.2h: `XMana`).
pub static CARD: CardDef =
    CardDef { spell: spell(&[targets::ANY], effect), x_mana: XMana::either(Mana::B, Mana::R), ..oracle::SOUL_BURN };
