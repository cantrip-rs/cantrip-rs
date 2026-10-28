use cantrip_api::*;
use cantrip_mechanics::{connive, effects, targets};
use cantrip_oracle as oracle;

/// "The owner of target creature an opponent controls puts it into their library second from the top or on the bottom. Then up
/// to one target creature you control connives." Each target is judged on its own as the spell resolves (CR 608.2b), so one
/// that became illegal is left alone and the other still happens; the spell is countered only if every target is illegal. The
/// instruction ends at "Then" (CR 608.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        effects::owner_puts_into_library_second_from_top_or_on_the_bottom(g, item, creature)?;
    }
    g.end_sentence();
    if let Some(creature) = targets::object(g, item, 1) {
        connive(g, creature, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE_AN_OPPONENT_CONTROLS.labelled("Its owner puts target creature an opponent controls into their library"),
            targets::CREATURE_YOU_CONTROL.optional().labelled("Up to one target creature you control connives"),
        ],
        effect,
    ),
    labels: &["Second from the top", "On the bottom"],
    ..oracle::TRICKSTERS_STRATAGEM
};
