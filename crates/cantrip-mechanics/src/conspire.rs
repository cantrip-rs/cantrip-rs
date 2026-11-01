//! Conspire (CR 702.78a): "As an additional cost to cast this spell, you may tap two untapped creatures you control that
//! share a color with it. When you do, copy it and you may choose new targets for the copy." A spell with several instances
//! of it pays each separately and each copies for its own payment (CR 702.78b), each with creatures of its own (CR 118.3).
//! Raiding Schemes ("Each noncreature spell you cast has conspire") gives it to spells; no printed card has it itself.
//!
//! The giving card lists `trigger(text)` among its `delayed` abilities, grants it with `statics::grant_delayed_ability`, and
//! gives two labels for the announcement ("cast without conspire", then paying it) as its `labels`.

use cantrip_api::*;

/// An untapped creature you control that shares a color with the spell being cast (CR 702.78a), read as the cost is paid.
fn untapped_creature_sharing_a_color(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller
        && !q.obj(id).tapped
        && q.chars(id).is(CardType::Creature)
        && q.chars(id).colors.intersects(q.chars(asker.source).colors)
}

/// Conspire's additional cost: "you may tap two untapped creatures you control that share a color with it".
pub const OPTIONAL: OptionalCost = OptionalCost::new(Cost::tap_untapped(2, untapped_creature_sharing_a_color));

/// "Copy it. You may choose new targets for the copy" (CR 707.10, 707.10c).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_spell(item.object, item.controller, true)?;
    Ok(())
}

/// The conspire ability a spell has from another object; `text` is the giving card's reminder text.
pub const fn trigger(text: &'static str) -> AbilityDef {
    paid_as_it_is_cast(OPTIONAL, copy_it, text)
}
