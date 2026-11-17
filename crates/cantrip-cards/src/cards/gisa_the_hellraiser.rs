use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

const TEXT_STATIC: &str = "Skeletons and Zombies you control get +1/+1 and have menace.";

/// "Skeletons and Zombies you control" (CR 613.1f, 205.3m): a Skeleton or a Zombie under the source's controller's control as the
/// earlier layers made it, the Hellraiser herself if she were one, the types read through the words of her text (CR 612.2).
fn skeletons_and_zombies_you_control(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_of_subtype::<{ lands::sub(Subtype::Skeleton) }>(view, source, affected, so_far)
        || statics::your_of_subtype::<{ lands::sub(Subtype::Zombie) }>(view, source, affected, so_far)
}

/// "Create two tapped 2/2 blue and black Zombie Rogue creature tokens" (CR 111.1): both at once, entering tapped.
fn create_two_tapped_zombie_rogues(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &tokens::ZOMBIE_ROGUE, 2, true)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE_ROGUE];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward_mana_and_life::<2, 2>("Ward\u{2014}{2}, Pay 2 life. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2} and 2 life.)"),
        triggered(triggers::YOU_COMMIT_A_CRIME, create_two_tapped_zombie_rogues, "Whenever you commit a crime, create two tapped 2/2 blue and black Zombie Rogue creature tokens. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)")
            .triggers_each_turn(1),
    ],
    statics: &[
        static_ability(Layer::PtModify, skeletons_and_zombies_you_control, statics::plus_pt::<1, 1>, TEXT_STATIC),
        static_ability(Layer::Ability, skeletons_and_zombies_you_control, statics::have_menace, TEXT_STATIC),
    ],
    ..oracle::GISA_THE_HELLRAISER
};
