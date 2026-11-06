use cantrip_api::*;
use cantrip_oracle as oracle;

/// An enchantment controlled by the caster, or an Aura attached to their permanent or an opposing attacker.
fn affected(q: &dyn Query, source: ObjectId, player: PlayerId, id: ObjectId) -> bool {
    if q.chars(id).is(CardType::Enchantment) && q.controller(id) == player {
        return true;
    }
    if !q.chars(id).has_subtype(q.words(source).subtype(Subtype::Aura)) {
        return false;
    }
    q.obj(id).attached_to.is_some_and(|host| {
        cantrip_mechanics::targets::permanent(q, host) && (q.controller(host) == player
            || (q.chars(host).is(CardType::Creature)
                && q.is_attacking(host)
                && q.state().opponents(player).any(|opponent| opponent == q.controller(host))))
    })
}

/// Return owned matching enchantments and Auras first, then destroy the others (CR 608.2, 701.8a, 400.7).
fn remove(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returned: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|&id| g.obj(id).owner == item.controller && affected(g, item.object, item.controller, id))
        .collect();
    g.return_to_hand(&returned);
    g.end_sentence();
    let others: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|id| !returned.contains(id) && affected(g, item.object, item.controller, *id))
        .collect();
    g.destroy_all(&others);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], remove),
    ..oracle::REMOVE_ENCHANTMENTS
};
