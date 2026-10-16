//! The check a method of a shell object makes: the caller holds a role, and is the owner of the
//! well-known name of the service that role is.

use peephole_bus::access_denied;
use peephole_bus::peephole_core::{Proof, Refused, Requirement, RoleName, Standing};
use zbus::fdo;
use zbus::message::Header;
use zbus::names::OwnedUniqueName;

use super::Peers;

/// How firmly a caller of a shell object is proved: its image, and its unit as the manager states
/// it. The desktop's own services run as units that unset the loader variables.
const PROOF: Proof = Proof::Full;

/// A service that may call, by the name it owns and the role it plays. `tag` is whatever the
/// object wants to know about which one it was.
#[derive(Debug, Clone)]
pub(crate) struct Claim<T> {
    /// The service's well-known bus name.
    pub name: &'static str,
    /// The role the table gives the program that owns it.
    pub role: RoleName,
    /// What the object keeps of the caller that passed.
    pub tag: T,
}

impl<T> Claim<T> {
    fn requirement(&self) -> Requirement {
        Requirement::new(self.role.clone(), PROOF)
    }
}

/// The caller of `header`, when it holds the role of one of `claims` and owns that claim's name;
/// anyone else is refused (`AccessDenied`). Returns the caller's unique name, which is the only
/// address an answer goes to, and the passing claim's tag.
pub(crate) async fn admit<T: Copy>(
    peers: &Peers,
    header: &Header<'_>,
    claims: &[Claim<T>],
) -> fdo::Result<(OwnedUniqueName, T)> {
    let sender = header.sender().map(|sender| sender.as_str());
    let refuse = |refused: Refused| {
        let wanted: Vec<&str> = claims.iter().map(|claim| claim.role.as_str()).collect();
        eprintln!("sill: callers: {sender:?} refused for {wanted:?}: {refused}");
        access_denied(refused)
    };
    let need: Vec<Requirement> = claims.iter().map(Claim::requirement).collect();
    let standing = peers.require(sender, &need).await.map_err(refuse)?;
    let claim = claim_held(&standing, claims).ok_or_else(|| refuse(Refused::NotAllowed))?;
    let owner = peers
        .owner_of(claim.name, &[claim.requirement()])
        .await
        .map_err(refuse)?;
    match Some(owner.as_str()) == sender {
        true => Ok((owner, claim.tag)),
        false => Err(refuse(Refused::NotAllowed)),
    }
}

/// The claim whose role `standing` holds.
fn claim_held<'c, T>(standing: &Standing, claims: &'c [Claim<T>]) -> Option<&'c Claim<T>> {
    let Standing::Service { role, .. } = standing else {
        return None;
    };
    claims.iter().find(|claim| claim.role == *role)
}
