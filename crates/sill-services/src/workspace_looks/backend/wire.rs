//! The property's wire type, `(ssssb)`, made from the model's.

use sill_model::workspace_looks::PublishedWorkspace;

/// One workspace: id, name, output, look, active.
pub(super) type WireWorkspace = (String, String, String, String, bool);

pub(super) fn wire(workspace: &PublishedWorkspace) -> WireWorkspace {
    (
        workspace.id.clone(),
        workspace.name.clone(),
        workspace.output.clone(),
        workspace.look.clone(),
        workspace.active,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Type;

    #[test]
    fn the_signature_is_the_documented_one() {
        assert_eq!(
            <Vec<WireWorkspace> as Type>::SIGNATURE.to_string(),
            "a(ssssb)"
        );
    }
}
