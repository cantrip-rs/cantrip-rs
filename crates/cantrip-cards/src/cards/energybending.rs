use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Lands you control gain all basic land types until end of turn. Draw a card." (CR 305.6, 611.2c, 613.1d): the lands its
/// controller controls as it resolves each gain Plains, Island, Swamp, Mountain and Forest in addition to their other types, and
/// with them the mana ability of each (CR 305.6); a land that enters later gains nothing. Then the controller draws.
fn lands_you_control_gain_all_basic_land_types_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    let basic_types = [Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain, Subtype::Forest].map(|t| g.words(item.object).subtype(t));
    instruction(g, |g| {
        for land in lands {
            for land_type in basic_types {
                g.add_effect(item.object, land, EffectKind::AddSubtype(land_type), Duration::EndOfTurn);
            }
        }
    });
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], lands_you_control_gain_all_basic_land_types_then_draw), ..oracle::ENERGYBENDING };
