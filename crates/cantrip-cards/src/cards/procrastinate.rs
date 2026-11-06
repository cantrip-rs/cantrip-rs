use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Twice X.
fn twice(x: u32) -> i32 {
    x as i32 * 2
}

/// "Tap target creature. Put twice X stun counters on it" (CR 701.26a, 122.1d): X as it was cast. The counters go on whether
/// or not the creature was untapped. With its target gone the spell does not resolve (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun,(twice(item.x)) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::PROCRASTINATE
};

#[cfg(test)]
mod tests {
    #[test]
    fn twice_x_is_exact_past_the_old_widths() {
        assert_eq!(super::twice(0), 0);
        assert_eq!(super::twice(16383), 32766);
        assert_eq!(super::twice(16384), 32768);
        assert_eq!(super::twice(65535), 131070);
    }
}
