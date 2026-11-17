use cantrip_api::*;
use cantrip_mechanics::{hand, lands, select, tokens, triggers};
use cantrip_oracle as oracle;

const BEAU_TEXT: &str = "Beau's power and toughness are each equal to the number of lands you control.";

/// Beau, the legendary blue Ox token Bonny Pall makes (CR 111.1, 205.4a), its power and toughness defined by the lands its
/// controller controls (CR 604.3, 613.4a).
pub static BEAU: CardDef = CardDef {
    name: "Beau",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Ox]),
    statics: &[characteristic_defining(Layer::PtDefine, lands::power_and_toughness_are_lands_you_control, BEAU_TEXT).lost_pt(0, 0)],
    text: BEAU_TEXT,
    ..CardDef::BLANK
};

/// "When Bonny Pall enters, create Beau, ..."
fn create_beau(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &BEAU, 1)?;
    Ok(())
}

/// "Draw a card, then you may put a land card from your hand or graveyard onto the battlefield" (CR 608.2c): the card drawn
/// can be the land put; putting it is not playing a land, so it needs no land drop (2024-04-12 ruling).
fn draw_then_may_put_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    hand::may_put_from_hand_or_graveyard_onto_battlefield(g, item.controller, item.object, select::LAND_CARD)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_beau,
            "When Bonny Pall enters, create Beau, a legendary blue Ox creature token with \"Beau's power and toughness are each equal to the number of lands you control.\"",
        ),
        triggered(
            triggers::YOU_ATTACK,
            draw_then_may_put_a_land,
            "Whenever you attack, draw a card, then you may put a land card from your hand or graveyard onto the battlefield.",
        ),
    ],
    ..oracle::BONNY_PALL_CLEARCUTTER
};

pub static EXTRA: &[&CardDef] = &[&BEAU];
