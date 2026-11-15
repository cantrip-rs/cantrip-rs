use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::you_have_delirium;
use cantrip_oracle as oracle;

/// This land becomes a copy of target permanent card in your graveyard until end of turn (CR 707.2): still the same object, so
/// it stays tapped or untapped.
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        let values = g.copiable_values(card);
        g.become_copy(item.object, item.object, values, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::lands::enters_tapped_unless_you_control::<{ cantrip_mechanics::lands::sub(Subtype::Forest) }>),
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        ability(cost("{2}{G}{G}"), become_copy, "Delirium \u{2014} {2}{G}{G}: This land becomes a copy of target permanent card in your graveyard until end of turn. Activate only if there are four or more card types among cards in your graveyard.")
            .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD])
            .condition(you_have_delirium),
    ],
    ..oracle::SHIFTING_WOODLAND
};
