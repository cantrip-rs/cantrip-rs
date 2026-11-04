use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_oracle as oracle;

/// "As long as The Lunar Whale attacked this turn" (CR 508.1, 611.2b): asked of the event history for this Vehicle; it stays
/// true after it leaves combat, to the end of the turn.
fn attacked_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().attacked_this_turn(asker.source)
}

/// Any spell, whatever its type or cost: "play the top card" is lands (`LibraryTopLands`) and spells for their mana costs.
fn any_spell(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2), and, "as long as The Lunar Whale attacked this
    // turn, you may play the top card of your library" (CR 305.1, 601.3): the land or spell, normal timing, normal costs.
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::While(attacked_this_turn, &[PlayRule::LibraryTopLands, PlayRule::LibraryTopSpells(any_spell, None)])],
    abilities: &[crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)")],
    ..oracle::THE_LUNAR_WHALE
};
