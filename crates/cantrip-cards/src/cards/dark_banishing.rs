use cantrip_api::*;
use cantrip_mechanics::destroy_target_no_regen;
use cantrip_oracle as oracle;

/// "Destroy target nonblack creature. It can't be regenerated." (CR 115.1).
fn destroy_target_nonblack_creature_cant_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .chars(id).colors.has(q.words(asker.source).color(Color::Black))) && q.chars(id)
        .is(CardType::Creature)))
    )
}

/// "Destroy target nonblack creature. It can't be regenerated." (CR 608.2, 701.8a).
fn destroy_target_nonblack_creature_cant(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    destroy_target_no_regen(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(destroy_target_nonblack_creature_cant_target_0)],
        destroy_target_nonblack_creature_cant,
    ),
    ..oracle::DARK_BANISHING
};
