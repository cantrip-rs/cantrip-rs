use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Draw a card if you control a Glimmer creature. If you don't control a Glimmer creature, create a 1/1 white Glimmer
/// enchantment creature token" (CR 608.2c, 608.2e): the Glimmer is looked for as the ability resolves, through the words of
/// the source's text (CR 612), and it is one choice made once, so the card and the token never both come.
fn draw_if_you_control_a_glimmer_otherwise_make_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let glimmer = g.words(item.object).subtype(Subtype::Glimmer);
    let controls_one = g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).is(CardType::Creature) && g.chars(id).has_subtype(glimmer));
    if controls_one {
        effects::draw(g, item, 1)
    } else {
        tokens::create(g, item.controller, &tokens::GLIMMER, 1)?;
        Ok(())
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_SECOND_MAIN_PHASE, draw_if_you_control_a_glimmer_otherwise_make_one,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, draw a card if you control a Glimmer creature. If you don't control a Glimmer creature, create a 1/1 white Glimmer enchantment creature token.")
        .condition(triggers::this_is_tapped)],
    ..oracle::GLIMMER_SEEKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::GLIMMER];
