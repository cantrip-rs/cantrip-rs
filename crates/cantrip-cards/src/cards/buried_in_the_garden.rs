use cantrip_api::*;
use cantrip_mechanics::mana::enchanted_permanent_tapped_for_mana;
use cantrip_mechanics::{choose_colors, exile_target_until_leaves_effect, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "... its controller adds an additional one mana of any color." A triggered mana ability (CR 605.1b), resolving at once
/// (CR 605.4a): the enchanted land's controller chooses the color and adds it, whoever controls the Aura.
fn enchanted_lands_controller_adds_one_mana_of_any_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(land) = item.subject else { return Ok(()) };
    let controller = g.controller(land);
    let colors = choose_colors(g, controller, item.object, ColorSet(0b11111), 1)?;
    g.add_mana(controller, colors[0].mana(), 1);
    Ok(())
}

/// "Enchant land" (CR 702.5a), "exile target nonland permanent you don't control until this Aura leaves the battlefield"
/// (CR 610.3, 603.1): the card returns, a new object, when the Aura leaves; if it has left before the ability resolves,
/// nothing is exiled at all (CR 610.3b).
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::LAND),
    abilities: &[
        triggered(
            THIS_ENTERS,
            exile_target_until_leaves_effect,
            "When this Aura enters, exile target nonland permanent you don't control until this Aura leaves the battlefield.",
        )
        .targets(&[targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS]),
        triggered(
            when(EventMask::TAPPED, enchanted_permanent_tapped_for_mana),
            enchanted_lands_controller_adds_one_mana_of_any_color,
            "Whenever enchanted land is tapped for mana, its controller adds an additional one mana of any color.",
        )
        .mana_ability(TriggeredMana::Unstated),
    ],
    ..oracle::BURIED_IN_THE_GARDEN
};
