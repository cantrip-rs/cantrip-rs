use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Counter the still-legal targeted spell (CR 608.2b, 701.6a).
fn counter_target_black_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{G}{G}"),
                counter_target_black_spell,
                "{G}{G}: Counter target black spell.",
            )
            .targets(&[targets::spell_of_either_color::<{ Color::Black as u8 }, { Color::Black as u8 }>()]),
    ],
    ..oracle::LIFEFORCE
};
