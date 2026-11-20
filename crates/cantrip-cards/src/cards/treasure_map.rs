use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those
/// counters, transform this artifact, and create three Treasure tokens." The Treasures are created whether or not the
/// transform did anything (CR 701.27f), together (CR 101.4), under the ability's controller.
fn scry_and_put_a_landmark_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::scry(g, item, 1)?;
    g.end_sentence();
    if effects::counter_then_transform_at_three::<{ CounterKind::Landmark as usize }>(g, item)? {
        tokens::create_by_rule(g, item.controller, &tokens::TREASURE, 3, false)?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(TAP.and(sacrifice(1, filters::treasure)), |g, item| effects::draw(g, item, 1), "{T}, Sacrifice a Treasure: Draw a card."),
    ],
    ..oracle::TREASURE_COVE
};

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}, {T}"),
        scry_and_put_a_landmark_counter,
        "{1}, {T}: Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those counters, transform this artifact, and create three Treasure tokens.",
    )],
    back: Some(&BACK),
    ..oracle::TREASURE_MAP
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
