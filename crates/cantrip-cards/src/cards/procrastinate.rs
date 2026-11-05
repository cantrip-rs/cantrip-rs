use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Twice X, as many as the counter's width holds (an X past 16383 doubles past an i16).
fn twice(x: u16) -> i16 {
    i16::try_from(u32::from(x) * 2).unwrap_or(i16::MAX)
}

/// "Tap target creature. Put twice X stun counters on it" (CR 701.26a, 122.1d): X as it was cast. The counters go on whether
/// or not the creature was untapped. With its target gone the spell does not resolve (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, i64::from(twice(item.x)));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::PROCRASTINATE
};

#[cfg(test)]
mod tests {
    #[test]
    fn twice_x_does_not_wrap_past_the_width_of_a_counter() {
        assert_eq!(super::twice(0), 0);
        assert_eq!(super::twice(16383), 32766);
        assert_eq!(super::twice(16384), i16::MAX);
        assert_eq!(super::twice(u16::MAX), i16::MAX);
    }
}
