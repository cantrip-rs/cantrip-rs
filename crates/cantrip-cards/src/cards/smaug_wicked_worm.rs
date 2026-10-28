use cantrip_api::*;
use cantrip_mechanics::{effects, spent, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create X tapped Treasure tokens, where X is the number of artifacts your opponents control" (CR 111.10a, 701.7a): counted as
/// the ability resolves, over every artifact any opponent controls as the layers make it (a land that is an artifact counts, a
/// Treasure of Smaug's own controller does not). The tokens are created together, tapped; none for a count of zero.
fn create_treasures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let x = opponents.into_iter().flat_map(|p| g.permanents_of(p)).filter(|&id| g.chars(id).is(CardType::Artifact)).count() as u32;
    tokens::create_by_rule(g, item.controller, &tokens::TREASURE, x, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_treasures,
            "When Smaug enters, create X tapped Treasure tokens, where X is the number of artifacts your opponents control.",
        ),
        // "Whenever you cast a spell, if mana from a Treasure was spent to cast it, you draw a card and lose 1 life": an intervening
        // "if" (CR 603.4) about a payment already made, which nothing can change.
        triggered(
            when_asked(EventMask::SPELL_CAST, spent::you_cast_a_spell_with_mana_from_a_treasure),
            effects::draw_a_card_and_lose_1_life,
            "Whenever you cast a spell, if mana from a Treasure was spent to cast it, you draw a card and lose 1 life.",
        ),
    ],
    // The mana a Treasure produces is told apart from other mana (CR 106.3): its payer chooses whether the spell is paid with it.
    mana_told_apart: &[spent::treasure_mana_is_told_apart],
    ..oracle::SMAUG_WICKED_WORM
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
