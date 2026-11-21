use cantrip_api::*;
use cantrip_mechanics::endure;
use cantrip_oracle as oracle;

/// "{X}{B}, {T}, Pay X life: This creature endures X." (CR 701.63a, 107.3a): X is announced as the ability is activated and sizes
/// the mana, the life paid (CR 119.4) and the endure. If Krumar has left the battlefield by now only the Spirit token is made.
fn endures_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    endure::endure(g, item.object, item.object, item.x as i32)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{X}{B}, {T}").and(Cost { pay_x_life: true, ..Cost::NONE }),
        endures_x,
        "{X}{B}, {T}, Pay X life: This creature endures X. Activate only as a sorcery. (Put X +1/+1 counters on it or create an X/X white Spirit creature token.)",
    )
    .sorcery_speed()],
    ..oracle::KRUMAR_INITIATE
};

pub static EXTRA: &[&CardDef] = &[&endure::SPIRIT];
