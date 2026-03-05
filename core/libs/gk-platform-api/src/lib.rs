use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateBundle {
    pub id: String,
    pub channel: String,
}

pub trait PlatformBackend {
    fn refresh_metadata(&self) -> Result<()>;
    fn download_bundle(&self, channel: &str) -> Result<UpdateBundle>;
    fn verify_bundle(&self, bundle: &UpdateBundle) -> Result<()>;
    fn stage(&self, bundle: &UpdateBundle) -> Result<()>;
    fn commit(&self) -> Result<()>;
    fn rollback(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend;

    impl PlatformBackend for MockBackend {
        fn refresh_metadata(&self) -> Result<()> {
            Ok(())
        }

        fn download_bundle(&self, channel: &str) -> Result<UpdateBundle> {
            Ok(UpdateBundle {
                id: "bundle-001".to_string(),
                channel: channel.to_string(),
            })
        }

        fn verify_bundle(&self, _bundle: &UpdateBundle) -> Result<()> {
            Ok(())
        }

        fn stage(&self, _bundle: &UpdateBundle) -> Result<()> {
            Ok(())
        }

        fn commit(&self) -> Result<()> {
            Ok(())
        }

        fn rollback(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn platform_backend_contract_is_callable() {
        let backend = MockBackend;
        backend.refresh_metadata().expect("metadata refresh ok");

        let bundle = backend
            .download_bundle("stable")
            .expect("bundle download should work");
        assert_eq!(bundle.channel, "stable");

        backend.verify_bundle(&bundle).expect("verify should work");
    }
}
