use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you commit a crime, put a loot counter on this artifact. This ability triggers only once each turn." (CR 700.13,
/// 603.2): the crime is the cast or activation that targets an opponent or something of theirs, so a Bolt at yourself is none.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOU_COMMIT_A_CRIME, effects::counters_on_self::<{ CounterKind::Loot as usize }, 1>, "Whenever you commit a crime, put a loot counter on this artifact. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)")
            .triggers_each_turn(1),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        ability(cost("{2}, {T}").and(remove_counters(CounterKind::Loot, 2)), |g, item| effects::draw(g, item, 1), "{2}, {T}, Remove two loot counters from this artifact: Draw a card."),
    ],
    ..oracle::BANDITS_HAUL
};
