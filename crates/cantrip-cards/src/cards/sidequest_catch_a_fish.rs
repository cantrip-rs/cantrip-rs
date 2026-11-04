use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, top_of_library, triggers};
use cantrip_oracle as oracle;

const REVEAL: Question = Question::new(
    "RevealArtifactOrCreatureToHand",
    "reveal it and put it into your hand?",
    "It is an artifact or creature card. If you do, you create a Food token and Sidequest: Catch a Fish transforms; if you don't, it stays on top of your library",
    "Reveal it",
    "Leave it on top",
);

/// "Look at the top card of your library. If it's an artifact or creature card, you may reveal it and put it into your hand.
/// If you put a card into your hand this way, create a Food token and transform this enchantment." (CR 701.20a, 111.10b,
/// 701.27a.) The card is looked at whatever it is, and may be left on top unrevealed even if it qualifies (the 2025-06-06
/// ruling); only a card that reached the hand makes the Food and the transformation. An empty library has nothing to look at.
fn catch_a_fish(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(&top) = top_of_library(g, you, 1).first() else { return Ok(()) };
    g.look_at(you, &[top]);
    g.end_sentence();
    let chars = g.chars(top);
    if !(chars.is(CardType::Artifact) || chars.is(CardType::Creature)) || !effects::ask_may(g, you, item.object, Purpose::Ask(&REVEAL))? {
        return Ok(());
    }
    g.reveal(you, &[top]);
    let [_] = g.return_to_hand_revealed(&[top])[..] else { return Ok(()) };
    g.end_sentence();
    tokens::create_food(g, you)?;
    effects::transform_self(g, item)
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        ability(cost("{3}, {T}").and(sacrifice(1, targets::filters::artifact)), effects::plus_one_counter_on_each_creature_you_control, "{3}, {T}, Sacrifice an artifact: Put a +1/+1 counter on each creature you control. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    ..oracle::COOKING_CAMPSITE
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        catch_a_fish,
        "At the beginning of your upkeep, look at the top card of your library. If it's an artifact or creature card, you may reveal it and put it into your hand. If you put a card into your hand this way, create a Food token and transform this enchantment.",
    )],
    back: Some(&BACK),
    ..oracle::SIDEQUEST_CATCH_A_FISH
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
