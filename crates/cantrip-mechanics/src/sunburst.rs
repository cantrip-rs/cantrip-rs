//! Sunburst (CR 702.44a), a static ability that functions as the object enters: "If this object is entering as a creature,
//! ignoring any type-changing effects that would affect it, it enters with a +1/+1 counter on it for each color of mana spent to
//! cast it. Otherwise, it enters with a charge counter on it for each color of mana spent to cast it." Colorless is not a color
//! (CR 105.1), and an object never cast spent none (CR 702.44b, 400.7). Engineered Explosives and Pentad Prism (charge
//! counters), and Arcbound Wanderer, Skyreach Manta and Suntouched Myr (+1/+1 counters) print it. The number also sets
//! another ability's (CR 702.44c): Arcbound Wanderer's "Modular-Sunburst".
//!
//! A card with sunburst has `enters: Some(sunburst::enters)`.

use cantrip_api::*;

/// How many colors of mana were spent to cast `object` (CR 702.44a, and converge, CR 207.2c: Prismatic Ending, Together as One,
/// Archaic's Agony, Magmablood Archaic, Snarl Song and Wildgrowth Archaic print it). A `Query`, not a `Game`, so a replacement
/// effect that reads the count, which has only that (Wildgrowth Archaic's), can ask it too.
pub fn colors_spent(q: &dyn Query, object: ObjectId) -> i16 {
    let spent = q.obj(object).spent;
    [Mana::W, Mana::U, Mana::B, Mana::R, Mana::G].iter().filter(|&&m| spent.get(m) > 0).count() as i16
}

/// The counters it enters with: +1/+1 counters if it is entering as a creature by its copiable values (type-changing effects
/// ignored), charge counters otherwise.
pub fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let colors = colors_spent(g, entering.object);
    let card = g.obj(entering.object).card;
    let kind = if g.token_chars(card, entering.copy).is(CardType::Creature) { CounterKind::PlusOne } else { CounterKind::Charge };
    entering.counters[kind as usize] += colors;
    Ok(())
}
