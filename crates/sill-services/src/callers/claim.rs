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
    /// What lets a caller in: the role, proved `Full`; and, in a build with `dev-callers`, the
    /// role at `Proof::Dev` as well, which only a stand-in named in the person's own rows file
    /// (`sill run --dev-callers`) has at that standing.
    pub(crate) fn requirements(&self) -> Vec<Requirement> {
        let real = Requirement::new(self.role.clone(), PROOF);
        #[cfg(feature = "dev-callers")]
        {
            vec![real, Requirement::dev(self.role.clone())]
        }
        #[cfg(not(feature = "dev-callers"))]
        {
            vec![real]
        }
    }
}

impl<T> Claim<T> {
    /// The unique name that owns this claim's well-known name, when its program holds the role:
    /// who an address by that name reaches. A name nobody owns, or a squatter, is refused.
    pub(crate) async fn owner(&self, peers: &Peers) -> Result<OwnedUniqueName, Refused> {
        peers.owner_of(self.name, &self.requirements()).await
    }
}

/// The caller of `header`, when it holds the role of one of `claims` and owns that claim's name;
/// anyone else is refused (`AccessDenied`) and the refusal is logged. Returns the caller's unique
/// name, which is the only address an answer goes to, and the passing claim's tag.
pub(crate) async fn admit<T: Copy>(
    peers: &Peers,
    header: &Header<'_>,
    claims: &[Claim<T>],
) -> fdo::Result<(OwnedUniqueName, T)> {
    check(peers, header, claims).await.map_err(|refused| {
        log_refusal(
            header.sender().map(zbus::names::UniqueName::as_str),
            claims,
            &refused,
        );
        access_denied(refused)
    })
}

/// [`admit`] without the log line or the error: why the caller of `header` is not let in, for a
/// caller that decides for itself whether a refusal is worth saying.
pub(crate) async fn check<T: Copy>(
    peers: &Peers,
    header: &Header<'_>,
    claims: &[Claim<T>],
) -> Result<(OwnedUniqueName, T), Refused> {
    let sender = header.sender().map(zbus::names::UniqueName::as_str);
    let need: Vec<Requirement> = claims.iter().flat_map(Claim::requirements).collect();
    let standing = peers.require(sender, &need).await?;
    let claim = claim_held(&standing, claims).ok_or(Refused::NotAllowed)?;
    let owner = peers.owner_of(claim.name, &claim.requirements()).await?;
    match Some(owner.as_str()) == sender {
        true => Ok((owner, claim.tag)),
        false => Err(Refused::NotAllowed),
    }
}

/// Say in the journal that `sender` was refused, and for which roles.
pub(crate) fn log_refusal<T>(sender: Option<&str>, claims: &[Claim<T>], refused: &Refused) {
    let wanted: Vec<&str> = claims.iter().map(|claim| claim.role.as_str()).collect();
    eprintln!("sill: callers: {sender:?} refused for {wanted:?}: {refused}");
}

/// The claim whose role `standing` holds.
fn claim_held<'c, T>(standing: &Standing, claims: &'c [Claim<T>]) -> Option<&'c Claim<T>> {
    let Standing::Service { role, .. } = standing else {
        return None;
    };
    claims.iter().find(|claim| claim.role == *role)
}
