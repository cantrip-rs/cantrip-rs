use cantrip_api::*;
use cantrip_mechanics::mana::{aura_enters_choosing_a_color, enchanted_permanent_tapped_for_mana, enchanted_permanents_controller_adds_the_chosen_color};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchant Forest" (CR 702.5a, 303.4a): a permanent that is a Forest now, Forest read as this Aura's text has the word
/// (CR 612.2). An Aura on a permanent that stops being a Forest - a nonbasic Forest under Blood Moon - is put into its
/// owner's graveyard (CR 303.4c, 704.5m).
fn forest(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let forest = q.words(asker.source).subtype(Subtype::Forest);
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.chars(id).has_subtype(forest))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(target(forest)),
    enters: Some(aura_enters_choosing_a_color),
    abilities: &[triggered(
        when(EventMask::TAPPED, enchanted_permanent_tapped_for_mana),
        enchanted_permanents_controller_adds_the_chosen_color,
        "Whenever enchanted Forest is tapped for mana, its controller adds an additional one mana of the chosen color.",
    )
    .mana_ability(TriggeredMana::ChosenColor)],
    ..oracle::UTOPIA_SPRAWL
};
