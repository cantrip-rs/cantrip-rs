use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{targets, tokens};
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Create a Treasure token. Until end of turn, you may look at the top card of your library any time, and you may play lands
/// and cast spells from the top of your library" (CR 111.10a, 611.2a, 401.2, 305.1, 601.3): the player is given the three play
/// rules for the turn, as effects of the resolved ability, so they outlast The Belligerent leaving the battlefield (they are the
/// player's, not the Vehicle's). Spells are cast for their mana costs, lands played as the turn's land play (CR 305.2).
fn treasure_then_look_and_play_from_the_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)?;
    let card = item.ability.card;
    instruction(g, |g| {
        for index in 0..3 {
            g.add_effect(item.object, item.object, EffectKind::PlayerPlayRule(AbilityRef::delayed(card, index)), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            treasure_then_look_and_play_from_the_top,
            "Whenever The Belligerent attacks, create a Treasure token. Until end of turn, you may look at the top card of your library any time, and you may play lands and cast spells from the top of your library.",
        ),
        crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    delayed: &[
        granted_play_rule(PlayRule::LookAtLibraryTop, "You may look at the top card of your library any time."),
        granted_play_rule(PlayRule::LibraryTopLands, "You may play lands from the top of your library."),
        granted_play_rule(PlayRule::LibraryTopSpells(targets::filters::any, None), "You may cast spells from the top of your library."),
    ],
    ..oracle::THE_BELLIGERENT
};
