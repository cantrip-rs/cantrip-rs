use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// I: "Create two 2/1 black Villain creature tokens with menace." (CR 111.1): both at once.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 2)?;
    Ok(())
}

/// II: "This Saga deals 2 damage to each non-Villain creature and each opponent." (CR 608.2c, 120.2): every creature on the
/// battlefield that is not a Villain and every opponent as the chapter resolves, all dealt together. "Villain" is a word of the
/// Saga's text, read through its words (CR 612).
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let villain = g.words(item.object).subtype(Subtype::Villain);
    let mut hits: Vec<(ObjectId, Target, u16)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) && !chars.has_subtype(villain)
        })
        .map(|id| (item.object, Target::Object(id), 2))
        .collect();
    hits.extend(g.state().opponents(item.controller).map(|p| (item.object, Target::Player(p), 2)));
    g.damage_together(&hits);
    Ok(())
}

/// III: "Create a Treasure token for each Villain you control." (CR 111.10a, 608.2h): the permanents you control that are a
/// Villain as the chapter resolves, a Villain that is no creature too, and all the Treasures at once (CR 101.4).
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let villain = g.words(item.object).subtype(Subtype::Villain);
    let count = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(villain)).count() as u32;
    tokens::create_by_rule(g, item.controller, &tokens::TREASURE, count, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Create two 2/1 black Villain creature tokens with menace."),
        chapter(2, chapter_two, "II \u{2014} This Saga deals 2 damage to each non-Villain creature and each opponent."),
        chapter(3, chapter_three, "III \u{2014} Create a Treasure token for each Villain you control."),
    ],
    ..oracle::AVENGERS_UNDER_SIEGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE, &tokens::TREASURE];
