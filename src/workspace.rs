/// Where commands operate: the personal workspace or an organisation, identified by `Id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace<Id> {
    Personal,
    Org(Id),
}

impl<Id> Workspace<Id> {
    /// `--personal` wins over `--org` (or its env), which wins over the configured default.
    pub fn pick(
        personal: bool,
        from_args: Option<Id>,
        from_config: Option<Id>,
    ) -> Self {
        match from_args.or(from_config) {
            Some(org_id) if !personal => Self::Org(org_id),
            _ => Self::Personal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Workspace;

    #[test]
    fn pick_precedence() {
        let (args, config) = (1, 2);
        let pick = Workspace::<u32>::pick;

        assert_eq!(pick(false, None, None), Workspace::Personal);
        assert_eq!(pick(false, None, Some(config)), Workspace::Org(config));
        assert_eq!(pick(false, Some(args), Some(config)), Workspace::Org(args));
        assert_eq!(pick(true, Some(args), Some(config)), Workspace::Personal);
    }
}
