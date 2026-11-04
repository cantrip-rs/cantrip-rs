use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// "{T}, Exile this artifact: Add {U}. When you next cast an instant or sorcery spell this turn, copy that spell. You may choose
/// new targets for the copy." (CR 605.3, 603.7b): a mana ability, so it does not use the stack and nothing responds to it; adding
/// the mana and setting up the card's delayed ability number 0 are one resolution. The delayed ability triggers once, on the next
/// instant or sorcery spell its controller casts this turn, whether or not that spell used the mana, and ends with the turn if none
/// is cast. The cost exiled the artifact, which is the delayed ability's source as it last was (CR 113.7a, 602.2b).
fn add_blue_and_copy_the_next_spell(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::U, 1);
    let this = g.card_id(&CARD);
    g.delayed_trigger(DelayedWhen::NextThisTurn, asker.source, asker.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP.and(EXILE_THIS),
        add(Mana::U).by(add_blue_and_copy_the_next_spell),
        "{T}, Exile this artifact: Add {U}. When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.",
    )],
    delayed: &[copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST],
    ..oracle::ETHER
};
