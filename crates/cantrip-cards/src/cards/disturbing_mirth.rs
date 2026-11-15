use cantrip_api::*;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "When this enchantment enters, you may sacrifice another enchantment or creature. If you do, draw two cards." (CR 603.6a,
/// 608.2d): declining is the empty answer to an up-to-one choice, nothing is asked when Disturbing Mirth is the only permanent its
/// controller has that fits (CR 701.21a), and the cards are drawn only for a permanent actually sacrificed (CR 608.2c).
fn sacrifice_another_to_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::another_creature_or_enchantment_permanents(select::up_to(1));
    if !effects::may_sacrifice(g, item.controller, item.object, rule)?.is_empty() {
        effects::draw(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_another_to_draw_two,
            "When this enchantment enters, you may sacrifice another enchantment or creature. If you do, draw two cards.",
        ),
        triggered(
            triggers::YOU_SACRIFICE_THIS,
            controller_manifests_dread,
            "When you sacrifice this enchantment, manifest dread. (Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)",
        )
        .on_sacrifice(),
    ],
    ..oracle::DISTURBING_MIRTH
};
