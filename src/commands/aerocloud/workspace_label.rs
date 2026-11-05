use crate::{
    aerocloud::{Client, Workspace, types::Id},
    commands::aerocloud::org,
    config::ConfigOrg,
};
use std::fmt;

/// Names a workspace for display.
#[derive(Debug, Clone)]
pub enum WorkspaceLabel {
    Personal,
    Org { id: Id, name: Option<String> },
}

impl WorkspaceLabel {
    /// Takes the organisation name from `saved` when it is the same organisation, otherwise
    /// looks it up, falling back to just its id if that fails.
    pub async fn resolve(
        client: &Client,
        workspace: &Workspace,
        saved: Option<&ConfigOrg>,
    ) -> Self {
        let Workspace::Org(org_id) = workspace else {
            return Self::Personal;
        };

        let name = if let Some(saved) =
            saved.filter(|saved| *saved.id == **org_id)
        {
            Some(saved.name.clone())
        } else {
            match org::list::fetch_all(client).await {
                Ok(orgs) => orgs
                    .into_iter()
                    .find(|org| *org.id == **org_id)
                    .map(|org| org.name),
                Err(err) => {
                    tracing::warn!("failed to fetch organisation name: {err}");
                    None
                }
            }
        };

        Self::Org {
            id: org_id.clone(),
            name,
        }
    }
}

impl fmt::Display for WorkspaceLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Personal => write!(f, "personal workspace"),
            Self::Org {
                id,
                name: Some(name),
            } => write!(f, "org {name} ({id})"),
            Self::Org { id, name: None } => write!(f, "org {id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceLabel;
    use crate::aerocloud::types::Id;
    use uuid::Uuid;

    #[test]
    fn display() {
        let id = Id(Uuid::from_u128(1));

        assert_eq!(WorkspaceLabel::Personal.to_string(), "personal workspace");
        assert_eq!(
            WorkspaceLabel::Org {
                id: id.clone(),
                name: Some("ACME Corp.".into())
            }
            .to_string(),
            "org ACME Corp. (00000000-0000-0000-0000-000000000001)"
        );
        assert_eq!(
            WorkspaceLabel::Org { id, name: None }.to_string(),
            "org 00000000-0000-0000-0000-000000000001"
        );
    }
}
