use cantrip_api::*;
use cantrip_oracle as oracle;

const fn golem(keyword: Keyword, text: &'static str) -> CardDef {
    CardDef {
        name: "Golem Token",
        token_name: Some(&[Subtype::Golem]),
        has_cost: false,
        types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
        subtypes: SubtypeSet::of(&[Subtype::Golem]),
        power: Some(3),
        toughness: Some(3),
        keywords: Keywords::of(&[keyword]),
        text,
        shares_name: true,
        ..CardDef::BLANK
    }
}

/// The three 3/3 colorless Golem artifact creature tokens, differing in the keyword each has.
pub static FLYING_GOLEM: CardDef = golem(Keyword::Flying, "Flying");
pub static VIGILANCE_GOLEM: CardDef = golem(Keyword::Vigilance, "Vigilance");
pub static TRAMPLE_GOLEM: CardDef = golem(Keyword::Trample, "Trample");

/// One instruction creating three differently keyworded tokens, so they are created together (CR 101.4, 111.1) and a
/// "one or more" creation trigger sees one event (CR 603.2c).
fn golems(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tokens = [g.card_id(&FLYING_GOLEM), g.card_id(&VIGILANCE_GOLEM), g.card_id(&TRAMPLE_GOLEM)];
    let tokens: Vec<(CardId, PlayerId)> = tokens.into_iter().map(|t| (t, item.controller)).collect();
    g.create_tokens_together(&tokens)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::THIS_DIES, golems, "When this creature dies, create a 3/3 colorless Golem artifact creature token with flying, a 3/3 colorless Golem artifact creature token with vigilance, and a 3/3 colorless Golem artifact creature token with trample.")],
    ..oracle::TRIPLICATE_TITAN
};

pub static EXTRA: &[&CardDef] = &[&FLYING_GOLEM, &VIGILANCE_GOLEM, &TRAMPLE_GOLEM];
